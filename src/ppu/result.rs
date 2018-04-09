#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PpuResult {
    Nmi,
    Draw,
    Scanline,
    None,
}
