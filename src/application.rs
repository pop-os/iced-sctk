use crate::{
    dpi::{LogicalSize, PhysicalPosition},
    egl::init_egl,
    error::{self, Error},
    event_loop::{
        self,
        control_flow::ControlFlow,
        proxy,
        state::{SctkLayerSurface, SctkPopup, SctkState, SctkWindow},
        SctkEventLoop,
    },
    sctk_event::{
        IcedSctkEvent, LayerSurfaceEventVariant, PopupEventVariant, SctkEvent, StartCause,
        WindowEventVariant, KeyboardEventVariant,
    },
    settings, Command, Debug, Executor, Runtime, Size, Subscription,
};
use futures::{channel::mpsc, task, Future, StreamExt, FutureExt};
use iced_native::{
    application::{self, StyleSheet},
    clipboard::{self, Null},
    command::platform_specific,
    mouse::{self, Interaction, ScrollDelta},
    widget::operation,
    Element, Renderer,
};

use sctk::{
    reexports::client::Proxy,
    seat::{
        keyboard::Modifiers,
        pointer::{PointerEvent, PointerEventKind},
    },
};
use std::{collections::HashMap, ffi::CString, fmt, marker::PhantomData, num::NonZeroU32};
use wayland_backend::client::ObjectId;

use glutin::{api::egl::context::PossiblyCurrentContext, prelude::*, surface::WindowSurface};
use iced_graphics::{compositor, renderer, window, Color, Point, Viewport};
use iced_native::user_interface::{self, UserInterface};
use iced_native::window::Id as SurfaceId;
use std::mem::ManuallyDrop;

#[derive(Debug)]
pub enum Event<Message> {
    /// A normal sctk event
    SctkEvent(IcedSctkEvent<Message>),
    /// TODO
    // Create a wrapper variant of `window::Event` type instead
    // (maybe we should also allow users to listen/react to those internal messages?)
    LayerSurface(platform_specific::wayland::layer_surface::Action<Message>),

    /// request sctk to set the cursor of the active pointer
    SetCursor(Interaction),
}

pub struct IcedSctkState;

/// An interactive, native cross-platform application.
///
/// This trait is the main entrypoint of Iced. Once implemented, you can run
/// your GUI application by simply calling [`run`]. It will run in
/// its own window.
///
/// An [`Application`] can execute asynchronous actions by returning a
/// [`Command`] in some of its methods.
///
/// When using an [`Application`] with the `debug` feature enabled, a debug view
/// can be toggled by pressing `F12`.
pub trait Application: Sized
where
    <Self::Renderer as crate::Renderer>::Theme: StyleSheet,
{
    /// The data needed to initialize your [`Application`].
    type Flags;

    /// The graphics backend to use to draw the [`Program`].
    type Renderer: Renderer;

    /// The type of __messages__ your [`Program`] will produce.
    type Message: std::fmt::Debug + Send;

    /// Handles a __message__ and updates the state of the [`Program`].
    ///
    /// This is where you define your __update logic__. All the __messages__,
    /// produced by either user interactions or commands, will be handled by
    /// this method.
    ///
    /// Any [`Command`] returned will be executed immediately in the
    /// background by shells.
    fn update(&mut self, message: Self::Message) -> Command<Self::Message>;

    /// Returns the widgets to display in the [`Application`].
    ///
    /// These widgets can produce __messages__ based on user interaction.
    fn view_window(
        &self,
        window: iced_native::window::Id,
    ) -> Element<'_, Self::Message, Self::Renderer>;

    /// Returns the widgets to display in the [`Application`].
    ///
    /// These widgets can produce __messages__ based on user interaction.
    fn view_popup(
        &self,
        window: iced_native::window::Id,
    ) -> Element<'_, Self::Message, Self::Renderer>;

    /// Returns the widgets to display in the [`Application`].
    ///
    /// These widgets can produce __messages__ based on user interaction.
    fn view_layer_surface(
        &self,
        window: iced_native::window::Id,
    ) -> Element<'_, Self::Message, Self::Renderer>;

    /// Initializes the [`Application`] with the flags provided to
    /// [`run`] as part of the [`Settings`].
    ///
    /// Here is where you should return the initial state of your app.
    ///
    /// Additionally, you can return a [`Command`] if you need to perform some
    /// async action in the background on startup. This is useful if you want to
    /// load state from a file, perform an initial HTTP request, etc.
    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>);

    /// Returns the current title of the [`Application`].
    ///
    /// This title can be dynamic! The runtime will automatically update the
    /// title of your application when necessary.
    fn title(&self) -> String;

    /// Returns the current [`Theme`] of the [`Application`].
    fn theme(&self) -> <Self::Renderer as crate::Renderer>::Theme;

    /// Returns the [`Style`] variation of the [`Theme`].
    fn style(&self) -> <<Self::Renderer as crate::Renderer>::Theme as StyleSheet>::Style {
        Default::default()
    }

    /// Returns the event `Subscription` for the current state of the
    /// application.
    ///
    /// The messages produced by the `Subscription` will be handled by
    /// [`update`](#tymethod.update).
    ///
    /// A `Subscription` will be kept alive as long as you keep returning it!
    ///
    /// By default, it returns an empty subscription.
    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::none()
    }

    /// Returns the scale factor of the [`Application`].
    ///
    /// It can be used to dynamically control the size of the UI at runtime
    /// (i.e. zooming).
    ///
    /// For instance, a scale factor of `2.0` will make widgets twice as big,
    /// while a scale factor of `0.5` will shrink them to half their size.
    ///
    /// By default, it returns `1.0`.
    fn scale_factor(&self) -> f64 {
        1.0
    }

    /// Returns whether the [`Application`] should be terminated.
    ///
    /// By default, it returns `false`.
    fn should_exit(&self) -> bool {
        false
    }

    /// TODO
    fn close_requested(&self, window: iced_native::window::Id) -> Self::Message;
}

/// Runs an [`Application`] with an executor, compositor, and the provided
/// settings.
pub fn run<A, E, C>(
    settings: settings::Settings<A::Flags>,
    compositor_settings: C::Settings,
) -> Result<(), error::Error>
where
    A: Application + 'static,
    E: Executor + 'static,
    C: window::GLCompositor<Renderer = A::Renderer> + 'static,
    <A::Renderer as iced_native::Renderer>::Theme: StyleSheet,
    A::Flags: Clone,
{
    let mut debug = Debug::new();
    debug.startup_started();

    let flags = settings.flags.clone();
    let exit_on_close_request = settings.exit_on_close_request;
    let is_layer_surface = matches!(settings.surface, settings::InitialSurface::LayerSurface(_));
    let mut event_loop =
        SctkEventLoop::<A::Message>::new(&settings).expect("Failed to initialize the event loop");

    let (id, surface) = match &settings.surface {
        settings::InitialSurface::LayerSurface(l) => event_loop.get_layer_surface(l.clone()),
        settings::InitialSurface::XdgWindow(_) => todo!(),
    };
    let init_id = surface.id();

    let surface_ids = HashMap::from([(init_id.clone(), id)]);

    let (runtime, ev_proxy) = {
        let ev_proxy = event_loop.proxy();
        let executor = E::new().map_err(Error::ExecutorCreationFailed)?;

        (Runtime::new(executor, ev_proxy.clone()), ev_proxy)
    };

    let (application, init_command) = {
        let flags = flags;

        runtime.enter(|| A::new(flags))
    };

    let windows: HashMap<SurfaceId, SctkWindow<A::Message>> = HashMap::new();
    let layer_surfaces: HashMap<SurfaceId, SctkLayerSurface<A::Message>> = HashMap::new();
    let popups: HashMap<SurfaceId, SctkPopup<A::Message>> = HashMap::new();

    let (_display, context, _config, surface) = init_egl(&surface, 100, 100);

    let gl_context = context.make_current(&surface).unwrap();
    let mut surfaces = HashMap::new();
    surfaces.insert(id, surface);

    #[allow(unsafe_code)]
    let (compositor, renderer) = unsafe {
        C::new(compositor_settings, |name| {
            let name = CString::new(name).unwrap();
            gl_context.get_proc_address(name.as_c_str())
        })?
    };
    let (mut sender, receiver) = mpsc::unbounded::<IcedSctkEvent<A::Message>>();

    let mut instance = Box::pin(run_instance::<A, E, C>(
        application,
        compositor,
        renderer,
        runtime,
        ev_proxy,
        debug,
        receiver,
        windows,
        layer_surfaces,
        popups,
        surfaces,
        surface_ids,
        gl_context,
        init_command,
        exit_on_close_request,
        if is_layer_surface {
            SurfaceIdWrapper::LayerSurface(id)
        } else {
            SurfaceIdWrapper::Window(id)
        },
    ));

    let mut context = task::Context::from_waker(task::noop_waker_ref());

    let _ = event_loop.run_return(move |event, event_loop, control_flow| {
        if let ControlFlow::ExitWithCode(_) = control_flow {
            return;
        }

        sender.start_send(event).expect("Send event");

        let poll = instance.as_mut().poll(&mut context);

        *control_flow = match poll {
            task::Poll::Pending => ControlFlow::Wait,
            task::Poll::Ready(_) => ControlFlow::ExitWithCode(1),
        };
    });

    Ok(())
}

async fn run_instance<A, E, C>(
    mut application: A,
    mut compositor: C,
    mut renderer: A::Renderer,
    mut runtime: Runtime<E, proxy::Proxy<Event<A::Message>>, Event<A::Message>>,
    mut ev_proxy: proxy::Proxy<Event<A::Message>>,
    mut debug: Debug,
    mut receiver: mpsc::UnboundedReceiver<IcedSctkEvent<A::Message>>,
    mut windows: HashMap<SurfaceId, SctkWindow<A::Message>>,
    mut layer_surfaces: HashMap<SurfaceId, SctkLayerSurface<A::Message>>,
    mut popups: HashMap<SurfaceId, SctkPopup<A::Message>>,
    mut surfaces: HashMap<SurfaceId, glutin::api::egl::surface::Surface<WindowSurface>>,
    mut surface_ids: HashMap<ObjectId, SurfaceId>,
    mut context: PossiblyCurrentContext,
    init_command: Command<A::Message>,
    exit_on_close_request: bool,
    init_id: SurfaceIdWrapper,
) -> Result<(), Error>
where
    A: Application + 'static,
    E: Executor + 'static,
    C: window::GLCompositor<Renderer = A::Renderer> + 'static,
    <A::Renderer as iced_native::Renderer>::Theme: StyleSheet,
{
    let mut cache = user_interface::Cache::default();

    let id = match init_id {
        SurfaceIdWrapper::LayerSurface(id) => id,
        SurfaceIdWrapper::Window(id) => id,
        SurfaceIdWrapper::Popup(id) => id,
    };
    let state = State::new(&application, init_id);

    let user_interface = build_user_interface(
        &application,
        user_interface::Cache::default(),
        &mut renderer,
        state.logical_size(),
        &mut debug,
        init_id,
    );
    let mut states = HashMap::from([(id, state)]);
    let mut interfaces = ManuallyDrop::new(HashMap::from([(id, user_interface)]));

    {
        let state = states.get(&id).unwrap();

        run_command(
            &application,
            &mut cache,
            state,
            &mut renderer,
            init_command,
            &mut runtime,
            &mut ev_proxy,
            &mut debug,
            || compositor.fetch_information(),
        );
    }

    let mut mouse_interaction = mouse::Interaction::default();
    let mut events: Vec<SctkEvent> = Vec::new();
    let mut messages: Vec<A::Message> = Vec::new();
    debug.startup_finished();

    let mut current_context_window = id;

    let mut surface_sizes = HashMap::from([(id, (100, 100))]);

    let kbd_surface_id: Option<ObjectId> = None;

    'main: while let Some(event) = receiver.next().await {
        match event {
            IcedSctkEvent::NewEvents(_) => {} // TODO Ashley: Seems to be ignored in iced_winit so i'll ignore for now
            IcedSctkEvent::UserEvent(_) => todo!(),
            IcedSctkEvent::SctkEvent(event) => match event {
                SctkEvent::SeatEvent { variant, .. } => todo!(),
                SctkEvent::PointerEvent { variant, .. } => {
                    let (state, native_id) = match surface_ids
                        .get(&variant.surface.id())
                        .and_then(|id| states.get_mut(id).map(|state| (state, id)))
                    {
                        Some(s) => s,
                        None => continue,
                    };
                    match variant.kind {
                        PointerEventKind::Enter { serial } => {
                            state.set_cursor_position(Point::new(
                                variant.position.0 as f32,
                                variant.position.1 as f32,
                            ));
                        }
                        PointerEventKind::Leave { serial } => {
                            state.set_cursor_position(Point::new(-1.0, -1.0));
                        }
                        PointerEventKind::Motion { time } => state.set_cursor_position(Point::new(
                            variant.position.0 as f32,
                            variant.position.1 as f32,
                        )),
                        PointerEventKind::Press {
                            time,
                            button,
                            serial,
                        } => todo!(),
                        PointerEventKind::Release {
                            time,
                            button,
                            serial,
                        } => todo!(),
                        PointerEventKind::Axis {
                            time,
                            horizontal,
                            vertical,
                            source,
                        } => todo!(),
                    }
                }
                SctkEvent::KeyboardEvent {
                    variant,
                    kbd_id,
                    seat_id,
                } => todo!(),
                SctkEvent::WindowEvent { variant, id } => todo!(),
                SctkEvent::LayerSurfaceEvent { variant, id } => match variant {
                    LayerSurfaceEventVariant::Created(_) => todo!(),
                    LayerSurfaceEventVariant::Done => todo!(),
                    LayerSurfaceEventVariant::Configure(configure) => {
                        if let Some(size) = surface_ids
                            .get(&id)
                            .and_then(|id| surface_sizes.get_mut(id))
                        {
                            *size = (configure.new_size.0, configure.new_size.1);
                        }
                    }
                },
                SctkEvent::PopupEvent {
                    variant,
                    toplevel_id,
                    parent_id,
                    id,
                } => todo!(),
                // TODO forward these events to an application which requests them?
                SctkEvent::NewOutput { id, info } => todo!(),
                SctkEvent::UpdateOutput { id, info } => todo!(),
                SctkEvent::RemovedOutput(_) => todo!(),
                SctkEvent::Draw(_) => unimplemented!(), // probably should never be forwarded here...
                SctkEvent::ScaleFactorChanged {
                    factor,
                    id,
                    inner_size,
                } => todo!(),
            },
            IcedSctkEvent::MainEventsCleared => {
                println!("Main events cleared");
                for (object_id, native_id) in &surface_ids {
                    println!("updating {:?}", native_id);
                    // returns (remove, copy)
                    let filter_events = |e: &SctkEvent| {
                        match e {
                            SctkEvent::SeatEvent { id, .. } => (id == object_id, false),
                            SctkEvent::PointerEvent { variant, .. } => (&variant.surface.id() == object_id, false),
                            SctkEvent::KeyboardEvent { .. } => (kbd_surface_id.as_ref() == Some(&object_id), false),
                            SctkEvent::WindowEvent { id, .. } => (id == object_id, false),
                            SctkEvent::LayerSurfaceEvent { id, .. } => (id == object_id, false),
                            SctkEvent::PopupEvent { id, .. } => (id == object_id, false),
                            SctkEvent::NewOutput {..} | SctkEvent::UpdateOutput { .. } | SctkEvent::RemovedOutput(_) => (false, true),
                            SctkEvent::Draw(_) => unimplemented!(),
                            SctkEvent::ScaleFactorChanged { id, .. } => (id == object_id, false),
                        }
                    };
                    let mut filtered = Vec::with_capacity(events.len());
                    let mut i = 0;

                    while i < events.len() {
                        let should_filter = filter_events(&mut events[i]);
                        if should_filter.0 {
                            filtered.push(events.remove(i));
                            // your code here
                        } else if should_filter.1 {
                            filtered.push(events[i].clone())
                        } else {
                            i += 1;
                        }
                    }
                    let cursor_position =
                        states.get(&id).unwrap().cursor_position();
                    if filtered.is_empty() && messages.is_empty() {
                        continue;
                    }
                    debug.event_processing_started();
                    let native_events: Vec<_> = filtered.into_iter().filter_map(|e| {
                        e.to_native()
                    }).collect();
                    let (interface_state, statuses) = {
                    let user_interface = interfaces.get_mut(&id).unwrap();
                        user_interface.update(
                            native_events.as_slice(), // TODO Ashley: pass filtered events & add platform specific events to iced_native
                            cursor_position,
                            &mut renderer,
                            &mut Null,
                            &mut messages,
                        )
                    };
                    debug.event_processing_finished();
                    for event in native_events.into_iter().zip(statuses.into_iter())
                    {
                        runtime.broadcast(event);
                    }


                    if !messages.is_empty()
                        || matches!(
                            interface_state,
                            user_interface::State::Outdated
                        )
                    {
                        let state = &mut states.get_mut(&id).unwrap();
                        let pure_states: HashMap<_, _> =
                            ManuallyDrop::into_inner(interfaces)
                                .drain()
                                .map(|(id, interface)| {
                                    (id, interface.into_cache())
                                })
                                .collect();

                        // Update application
                        update(
                            &mut application,
                            &mut cache,
                            state,
                            &mut renderer,
                            &mut runtime,
                            &mut ev_proxy,
                            &mut debug,
                            &mut messages,
                            || compositor.fetch_information(),
                        );

                        // Update window
                        state.synchronize(&application);

                        let should_exit = application.should_exit();

                        interfaces = ManuallyDrop::new(build_user_interfaces(
                            &application,
                            &mut renderer,
                            &mut debug,
                            &states,
                            pure_states,
                        ));

                        if should_exit {
                            break 'main;
                        }
                    }
                }
            }
            IcedSctkEvent::RedrawRequested(id) => {
                if let Some((
                    native_id,
                    Some(size),
                    Some(egl_surface),
                    Some(mut user_interface),
                    Some(state),
                )) = surface_ids.get(&id).map(|id| {
                    let window = surface_sizes.get_mut(id);
                    let surface = surfaces.get_mut(id);
                    let interface = interfaces.remove(id);
                    let state = states.get_mut(id);
                    (*id, window, surface, interface, state)
                }) {
                    println!("Rredrawing: {:?}", native_id);
                    debug.render_started();

                    if current_context_window != native_id {
                        if context.make_current(egl_surface).is_ok() {
                            current_context_window = native_id;
                        } else {
                            continue;
                        }
                    }

                    if state.viewport_changed() {
                        let physical_size = state.physical_size();
                        let logical_size = state.logical_size();

                        debug.layout_started();
                        user_interface = user_interface.relayout(logical_size, &mut renderer);
                        debug.layout_finished();

                        debug.draw_started();
                        let new_mouse_interaction = user_interface.draw(
                            &mut renderer,
                            state.theme(),
                            &renderer::Style {
                                text_color: state.text_color(),
                            },
                            state.cursor_position(),
                        );
                        debug.draw_finished();
                        ev_proxy.send_event(Event::SetCursor(new_mouse_interaction));

                        egl_surface.resize(
                            &context,
                            NonZeroU32::new(physical_size.width).unwrap(),
                            NonZeroU32::new(physical_size.height).unwrap(),
                        );

                        compositor.resize_viewport(physical_size);

                        let _ = interfaces.insert(native_id, user_interface);
                    }

                    compositor.present(
                        &mut renderer,
                        state.viewport(),
                        state.background_color(),
                        &debug.overlay(),
                    );
                    let _ = egl_surface.swap_buffers(&context);

                    debug.render_finished();
                }
            }
            IcedSctkEvent::RedrawEventsCleared => {
                // TODO
            }
            IcedSctkEvent::LoopDestroyed => todo!(),
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub enum SurfaceIdWrapper {
    LayerSurface(SurfaceId),
    Window(SurfaceId),
    Popup(SurfaceId),
}
/// Builds a [`UserInterface`] for the provided [`Application`], logging
/// [`struct@Debug`] information accordingly.
pub fn build_user_interface<'a, A: Application>(
    application: &'a A,
    cache: user_interface::Cache,
    renderer: &mut A::Renderer,
    size: Size,
    debug: &mut Debug,
    id: SurfaceIdWrapper,
) -> UserInterface<'a, A::Message, A::Renderer>
where
    <A::Renderer as crate::Renderer>::Theme: StyleSheet,
{
    debug.view_started();
    let view = match id {
        SurfaceIdWrapper::LayerSurface(id) => application.view_layer_surface(id),
        SurfaceIdWrapper::Window(id) => application.view_window(id),
        SurfaceIdWrapper::Popup(id) => application.view_popup(id),
    };
    debug.view_finished();

    debug.layout_started();
    let user_interface = UserInterface::build(view, size, cache, renderer);
    debug.layout_finished();

    user_interface
}

/// The state of a surface created by the application [`Application`].
#[allow(missing_debug_implementations)]
pub struct State<A: Application>
where
    <A::Renderer as crate::Renderer>::Theme: application::StyleSheet,
{
    pub(crate) id: SurfaceIdWrapper,
    title: String,
    scale_factor: f64,
    viewport: Viewport,
    viewport_changed: bool,
    cursor_position: Point,
    modifiers: Modifiers,
    theme: <A::Renderer as crate::Renderer>::Theme,
    appearance: application::Appearance,
    application: PhantomData<A>,
}

impl<A: Application> State<A>
where
    <A::Renderer as crate::Renderer>::Theme: application::StyleSheet,
{
    /// Creates a new [`State`] for the provided [`Application`]
    pub fn new(application: &A, id: SurfaceIdWrapper) -> Self {
        let title = application.title();
        let scale_factor = application.scale_factor();
        let theme = application.theme();
        let appearance = theme.appearance(application.style());

        let viewport = Viewport::with_physical_size(Size::new(1, 1), 1.0);

        Self {
            id,
            title,
            scale_factor,
            viewport,
            viewport_changed: false,
            // TODO: Encode cursor availability in the type-system
            cursor_position: Point::new(-1.0, -1.0),
            modifiers: Modifiers::default(),
            theme,
            appearance,
            application: PhantomData,
        }
    }

    /// Returns the current [`Viewport`] of the [`State`].
    pub fn viewport(&self) -> &Viewport {
        &self.viewport
    }

    /// TODO
    pub fn viewport_changed(&self) -> bool {
        self.viewport_changed
    }

    /// Returns the physical [`Size`] of the [`Viewport`] of the [`State`].
    pub fn physical_size(&self) -> Size<u32> {
        self.viewport.physical_size()
    }

    /// Returns the logical [`Size`] of the [`Viewport`] of the [`State`].
    pub fn logical_size(&self) -> Size<f32> {
        self.viewport.logical_size()
    }

    /// Returns the current scale factor of the [`Viewport`] of the [`State`].
    pub fn scale_factor(&self) -> f64 {
        self.viewport.scale_factor()
    }

    /// Returns the current cursor position of the [`State`].
    pub fn cursor_position(&self) -> Point {
        self.cursor_position
    }

    /// Returns the current keyboard modifiers of the [`State`].
    pub fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    /// Returns the current theme of the [`State`].
    pub fn theme(&self) -> &<A::Renderer as crate::Renderer>::Theme {
        &self.theme
    }

    /// Returns the current background [`Color`] of the [`State`].
    pub fn background_color(&self) -> Color {
        self.appearance.background_color
    }

    /// Returns the current text [`Color`] of the [`State`].
    pub fn text_color(&self) -> Color {
        self.appearance.text_color
    }

    pub fn set_cursor_position(&mut self, p: Point) {
        self.cursor_position = p;
    }

    /// Processes the provided window event and updates the [`State`]
    /// accordingly.
    pub(crate) fn update_window(
        &mut self,
        window: &SctkWindow<A::Message>,
        event: &WindowEventVariant,
        _debug: &mut Debug,
    ) {
        todo!()
    }

    /// Processes the provided layer surface event and updates the [`State`]
    /// accordingly.
    pub(crate) fn update_layer_surface(
        &mut self,
        layer_surface: &SctkLayerSurface<A::Message>,
        event: &LayerSurfaceEventVariant,
        _debug: &mut Debug,
    ) {
        todo!()
    }

    /// Processes the provided popup event and updates the [`State`]
    /// accordingly.
    pub(crate) fn update_popup(
        &mut self,
        popup: &SctkPopup<A::Message>,
        event: &PopupEventVariant,
        _debug: &mut Debug,
    ) {
        todo!()
    }

    /// Synchronizes the [`State`] with its [`Application`] and its respective
    /// windows.
    ///
    /// Normally an [`Application`] should be synchronized with its [`State`]
    /// and window after calling [`Application::update`].
    ///
    /// [`Application::update`]: crate::Program::update
    pub(crate) fn synchronize_window(
        &mut self,
        application: &A,
        window: &SctkWindow<A::Message>,
        proxy: &proxy::Proxy<Event<A::Message>>,
    ) {
        self.synchronize(application);
    }

    /// Synchronizes the [`State`] with its [`Application`] and its respective
    /// windows.
    ///
    /// Normally an [`Application`] should be synchronized with its [`State`]
    /// and window after calling [`Application::update`].
    ///
    /// [`Application::update`]: crate::Program::update
    pub(crate) fn synchronize_layer_surface(
        &mut self,
        application: &A,
        window: &SctkPopup<A::Message>,
        proxy: &proxy::Proxy<Event<A::Message>>,
    ) {
        self.synchronize(application);
    }

    /// Synchronizes the [`State`] with its [`Application`] and its respective
    /// windows.
    ///
    /// Normally an [`Application`] should be synchronized with its [`State`]
    /// and window after calling [`Application::update`].
    ///
    /// [`Application::update`]: crate::Program::update
    pub(crate) fn synchronize_popup(
        &mut self,
        application: &A,
        window: &SctkPopup<A::Message>,
        proxy: &proxy::Proxy<Event<A::Message>>,
    ) {
        self.synchronize(application);
    }

    fn synchronize(&mut self, application: &A) {
        // Update theme and appearance
        self.theme = application.theme();
        self.appearance = self.theme.appearance(application.style());
    }
}

/// Updates an [`Application`] by feeding it the provided messages, spawning any
/// resulting [`Command`], and tracking its [`Subscription`].
pub(crate) fn update<A: Application, E: Executor>(
    application: &mut A,
    cache: &mut user_interface::Cache,
    state: &State<A>,
    renderer: &mut A::Renderer,
    runtime: &mut Runtime<E, proxy::Proxy<Event<A::Message>>, Event<A::Message>>,
    proxy: &mut proxy::Proxy<Event<A::Message>>,
    debug: &mut Debug,
    messages: &mut Vec<A::Message>,
    graphics_info: impl FnOnce() -> compositor::Information + Copy,
) where
    <A::Renderer as crate::Renderer>::Theme: StyleSheet,
{
    for message in messages.drain(..) {
        debug.log_message(&message);

        debug.update_started();
        let command = runtime.enter(|| application.update(message));
        debug.update_finished();

        run_command(
            application,
            cache,
            state,
            renderer,
            command,
            runtime,
            proxy,
            debug,
            graphics_info,
        );
    }

    let subscription = application
        .subscription()
        .map(|m| Event::SctkEvent(IcedSctkEvent::UserEvent(m)));
    runtime.track(subscription);
}

/// Runs the actions of a [`Command`].
fn run_command<A, E>(
    application: &A,
    cache: &mut user_interface::Cache,
    state: &State<A>,
    renderer: &mut A::Renderer,
    command: Command<A::Message>,
    runtime: &mut Runtime<E, proxy::Proxy<Event<A::Message>>, Event<A::Message>>,
    proxy: &mut proxy::Proxy<Event<A::Message>>,
    debug: &mut Debug,
    _graphics_info: impl FnOnce() -> compositor::Information + Copy,
) where
    A: Application,
    E: Executor,
    <A::Renderer as crate::Renderer>::Theme: StyleSheet,
{
    use iced_native::command;
    use iced_native::system;

    let id = &state.id;

    for action in command.actions() {
        match action {
            command::Action::Future(future) => {
                runtime.spawn(Box::pin(future.map(|e| Event::SctkEvent(IcedSctkEvent::UserEvent(e)))));
            }
            command::Action::Clipboard(action) => match action {
                clipboard::Action::Read(tag) => {
                    todo!();
                }
                clipboard::Action::Write(contents) => {
                    todo!();
                }
            },
            command::Action::Window(id, action) => {
                todo!()
            }
            command::Action::System(action) => match action {
                system::Action::QueryInformation(_tag) => {
                    #[cfg(feature = "system")]
                    {
                        let graphics_info = _graphics_info();
                        let proxy = proxy.clone();

                        let _ = std::thread::spawn(move || {
                            let information = crate::system::information(graphics_info);

                            let message = _tag(information);

                            proxy
                                .send_event(Event::Application(message))
                                .expect("Send message to event loop")
                        });
                    }
                }
            },
            command::Action::Widget(action) => {
                let mut current_cache = std::mem::take(cache);
                let mut current_operation = Some(action.into_operation());

                let mut user_interface = build_user_interface(
                    application,
                    current_cache,
                    renderer,
                    state.logical_size(),
                    debug,
                    id.clone(), // TODO: run the operation on every widget tree
                );

                while let Some(mut operation) = current_operation.take() {
                    user_interface.operate(renderer, operation.as_mut());

                    match operation.finish() {
                        operation::Outcome::None => {}
                        operation::Outcome::Some(message) => {
                            proxy.send_event(Event::SctkEvent(IcedSctkEvent::UserEvent(message)));
                        }
                        operation::Outcome::Chain(next) => {
                            current_operation = Some(next);
                        }
                    }
                }

                current_cache = user_interface.into_cache();
                *cache = current_cache;
            }
            command::Action::PlatformSpecific(platform_specific::Action::Wayland(
                platform_specific::wayland::Action::LayerSurface(layer_surface_action),
            )) => {
                proxy.send_event(Event::LayerSurface(layer_surface_action));
            }
            command::Action::PlatformSpecific(platform_specific::Action::Wayland(
                platform_specific::wayland::Action::Window(window_action),
            )) => match window_action {
                platform_specific::wayland::window::Action::Window { builder, .. } => todo!(),
            },
            _ => {}
        }
    }
}

pub fn build_user_interfaces<'a, A>(
    application: &'a A,
    renderer: &mut A::Renderer,
    debug: &mut Debug,
    states: &HashMap<SurfaceId, State<A>>,
    mut pure_states: HashMap<SurfaceId, user_interface::Cache>,
) -> HashMap<SurfaceId, UserInterface<'a, <A as Application>::Message, <A as Application>::Renderer>>
where
    A: Application + 'static,
    <A::Renderer as crate::Renderer>::Theme: StyleSheet,
{
    let mut interfaces = HashMap::new();

    for (id, pure_state) in pure_states.drain() {
        let state = &states.get(&id).unwrap();

        let user_interface = build_user_interface(
            application,
            pure_state,
            renderer,
            state.logical_size(),
            debug,
            state.id,
        );

        let _ = interfaces.insert(id, user_interface);
    }

    interfaces
}

pub fn run_event_loop<T, F>(
    mut event_loop: event_loop::SctkEventLoop<T>,
    event_handler: F,
) -> Result<(), crate::error::Error>
where
    F: 'static + FnMut(IcedSctkEvent<T>, &SctkState<T>, &mut ControlFlow),
    T: 'static + fmt::Debug,
{
    let _ = event_loop.run_return(event_handler);

    Ok(())
}
