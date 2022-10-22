use crate::{
    dpi::PhysicalPosition,
    egl::init_egl,
    error::{self, Error},
    event_loop::{
        self,
        control_flow::ControlFlow,
        proxy,
        state::{SctkLayerSurface, SctkPopup, SctkState, SctkWindow},
        SctkEventLoop,
    },
    sctk_event::{IcedSctkEvent, LayerSurfaceEventVariant, PopupEventVariant, WindowEventVariant},
    settings, Command, Debug, Executor, Runtime, Size, Subscription,
};
use futures::{channel::mpsc, task, Future, StreamExt};
use iced_native::{
    application::{self, StyleSheet},
    clipboard,
    command::platform_specific,
    mouse,
    widget::operation,
    Element, Renderer,
};

use sctk::{reexports::client::Proxy, seat::keyboard::Modifiers};
use std::{collections::HashMap, ffi::CString, fmt, marker::PhantomData};
use wayland_backend::client::ObjectId;

use glutin::{api::egl::context::PossiblyCurrentContext, prelude::*, surface::WindowSurface};
use iced_graphics::{compositor, window, Color, Point, Viewport};
use iced_native::user_interface::{self, UserInterface};
use iced_native::window::Id as SurfaceId;
use std::mem::ManuallyDrop;

#[derive(Debug)]
pub enum Event<Message> {
    /// An [`Application`] generated message
    Application(Message),

    /// TODO
    // Create a wrapper variant of `window::Event` type instead
    // (maybe we should also allow users to listen/react to those internal messages?)
    LayerSurface(platform_specific::wayland::layer_surface::Action<Message>),
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
    let (mut sender, receiver) = mpsc::unbounded::<IcedSctkEvent<A::Message>>();

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

    let mut surface_ids = HashMap::from([(id, init_id.clone())]);

    let id = SurfaceId::new(&init_id);
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

    let (display, context, config, surface) = init_egl(&surface, 1, 1);

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
    let (mut sender, receiver) = mpsc::unbounded::<Event<A::Message>>();

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
        let event = match event {
            IcedSctkEvent::NewEvents(_) => todo!(),
            IcedSctkEvent::UserEvent(event) => Some(event),
            IcedSctkEvent::SctkEvent(_) => todo!(),
            IcedSctkEvent::MainEventsCleared => todo!(),
            IcedSctkEvent::RedrawRequested(_) => todo!(),
            IcedSctkEvent::RedrawEventsCleared => todo!(),
            IcedSctkEvent::LoopDestroyed => todo!(),
        };
        if let Some(event) = event {
            sender
                .start_send(Event::Application(event))
                .expect("Send event");

            let poll = instance.as_mut().poll(&mut context);

            *control_flow = match poll {
                task::Poll::Pending => ControlFlow::Wait,
                task::Poll::Ready(_) => ControlFlow::ExitWithCode(1),
            };
        }
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
    mut receiver: mpsc::UnboundedReceiver<Event<A::Message>>,
    mut windows: HashMap<SurfaceId, SctkWindow<A::Message>>,
    mut layer_surfaces: HashMap<SurfaceId, SctkLayerSurface<A::Message>>,
    mut popups: HashMap<SurfaceId, SctkPopup<A::Message>>,
    mut surfaces: HashMap<SurfaceId, glutin::api::egl::surface::Surface<WindowSurface>>,
    mut surface_ids: HashMap<SurfaceId, ObjectId>,
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
            &mut windows,
            &mut layer_surfaces,
            &mut popups,
            || compositor.fetch_information(),
        );
    }

    let mut mouse_interaction = mouse::Interaction::default();
    let mut events: Vec<IcedSctkEvent<A::Message>> = Vec::new();
    let mut messages: Vec<A::Message> = Vec::new();

    debug.startup_finished();

    while let Some(event) = receiver.next().await {
        match event {
            Event::Application(_) => todo!(),
            Event::LayerSurface(_) => todo!(),
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
    cursor_position: PhysicalPosition<f64>,
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
            cursor_position: PhysicalPosition::new(-1.0, -1.0),
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
        todo!()
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
    windows: &mut HashMap<SurfaceId, SctkWindow<A::Message>>,
    layer_surfaces: &mut HashMap<SurfaceId, SctkLayerSurface<A::Message>>,
    popups: &mut HashMap<SurfaceId, SctkPopup<A::Message>>,
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
            windows,
            layer_surfaces,
            popups,
            graphics_info,
        );
    }

    let subscription = application.subscription().map(|m| Event::Application(m));
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
    windows: &mut HashMap<SurfaceId, SctkWindow<A::Message>>,
    layer_surfaces: &mut HashMap<SurfaceId, SctkLayerSurface<A::Message>>,
    popups: &mut HashMap<SurfaceId, SctkPopup<A::Message>>,
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
                // runtime.spawn(Box::pin(future.map(Event::Application)));
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
                            proxy.send_event(Event::Application(message));
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
