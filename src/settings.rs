pub struct Settings<Flags> {
    /// The data needed to initialize an [`Application`].
    ///
    /// [`Application`]: crate::Application
    pub flags: Flags,
    /// optional keyboard repetition config
    kbd_repeat: Option<u32>,
    /// optional name and size of a custom pointer theme
    ptr_theme: Option<(String, u32)>,
}
