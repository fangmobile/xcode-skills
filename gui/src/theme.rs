use gpui::{Rgba, WindowAppearance, rgb};

#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Rgba,
    pub panel: Rgba,
    pub panel_hover: Rgba,
    pub selected: Rgba,
    pub border: Rgba,
    pub text: Rgba,
    pub muted: Rgba,
    pub accent: Rgba,
    pub success: Rgba,
    pub warning: Rgba,
    pub danger: Rgba,
}

impl Palette {
    pub fn from_appearance(appearance: WindowAppearance) -> Self {
        match appearance {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Self {
                bg: rgb(0x1c1c1e),
                panel: rgb(0x2c2c2e),
                panel_hover: rgb(0x3a3a3c),
                selected: rgb(0x2f3b4a),
                border: rgb(0x3a3a3c),
                text: rgb(0xe5e5ea),
                muted: rgb(0x8e8e93),
                accent: rgb(0x0a84ff),
                success: rgb(0x30d158),
                warning: rgb(0xff9f0a),
                danger: rgb(0xff453a),
            },
            _ => Self {
                bg: rgb(0xf2f2f7),
                panel: rgb(0xffffff),
                panel_hover: rgb(0xe8e8ed),
                selected: rgb(0xdde9f7),
                border: rgb(0xd1d1d6),
                text: rgb(0x1c1c1e),
                muted: rgb(0x6c6c70),
                accent: rgb(0x007aff),
                success: rgb(0x248a3d),
                warning: rgb(0xb25000),
                danger: rgb(0xd70015),
            },
        }
    }
}
