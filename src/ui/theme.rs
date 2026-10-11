//! Farben der Oberfläche (dunkles Thema).

pub const PAGE: u32 = 0x0d0d0d;
pub const SURFACE: u32 = 0x1a1a19;
pub const SURFACE_RAISED: u32 = 0x262624;
pub const TEXT: u32 = 0xffffff;
pub const TEXT_SECONDARY: u32 = 0xc3c2b7;
pub const TEXT_MUTED: u32 = 0x898781;
pub const GRID: u32 = 0x2c2c2a;
pub const AXIS: u32 = 0x383835;
pub const BORDER: u32 = 0x2e2e2c;

// Feste Zuordnung Kennzahl → Farbe, überall in der App gleich.
pub const WEIGHT: u32 = 0x3987e5;
pub const FAT: u32 = 0xd95926;
pub const MUSCLE: u32 = 0x199e70;
pub const WATER: u32 = 0xc98500;
pub const BONE: u32 = 0x898781;

// Statusfarben, nur für Bewertungen und immer mit Text.
pub const GOOD: u32 = 0x0ca30c;
pub const WARNING: u32 = 0xfab219;
pub const SERIOUS: u32 = 0xec835a;
pub const CRITICAL: u32 = 0xd03b3b;
