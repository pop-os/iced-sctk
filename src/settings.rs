use iced_native::command::platform_specific::wayland::layer_surface::IcedLayerSurface;
use sctk::shell::{
    xdg::window::WindowBuilder,
};

#[derive(Debug)]
pub struct Settings<Flags> {
    /// The data needed to initialize an [`Application`].
    ///
    /// [`Application`]: crate::Application
    pub flags: Flags,
    /// optional keyboard repetition config
    pub kbd_repeat: Option<u32>,
    /// optional name and size of a custom pointer theme
    pub ptr_theme: Option<(String, u32)>,
    /// surface
    pub surface: InitialSurface,
    /// whether the application should exit on close of all windows
    pub exit_on_close_request: bool,
}

#[derive(Debug)]
pub enum InitialSurface {
    LayerSurface(IcedLayerSurface),
    XdgWindow(WindowBuilder),
}

impl Default for InitialSurface {
    fn default() -> Self {
        Self::LayerSurface(IcedLayerSurface::default())
    }
}
