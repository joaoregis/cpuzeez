//! `cpuzeez.ini`, same keys and meaning as `cpuz.ini` (see FEATURES.md 1.9).

#[derive(Debug, Clone)]
pub struct Config {
    pub acpi: bool,
    pub pci: bool,
    pub max_pci_bus: u16,
    pub dmi: bool,
    pub sensor: bool,
    pub smbus: bool,
    pub display: bool,
    pub use_display_api: bool,
    pub bus_clock: bool,
    pub chipset: bool,
    pub spd: bool,
    pub lpcio: bool,
    pub smart: bool,
    pub xoc: bool,
    pub check_updates: bool,
    pub text_font_name: String,
    pub text_font_size: f32,
    pub text_font_color: u32,
    pub label_font_name: String,
    pub label_font_size: f32,
    pub label_font_color: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            acpi: true,
            pci: true,
            max_pci_bus: 256,
            dmi: true,
            sensor: true,
            smbus: true,
            display: true,
            use_display_api: true,
            bus_clock: true,
            chipset: true,
            spd: true,
            lpcio: true,
            smart: true,
            xoc: false,
            check_updates: false,
            text_font_name: String::new(),
            text_font_size: 0.0,
            text_font_color: 0x0000A0,
            label_font_name: String::new(),
            label_font_size: 0.0,
            label_font_color: 0x323232,
        }
    }
}

impl Config {
    /// Next to the executable, like CPU-Z. Missing file = defaults.
    pub fn load() -> Config {
        let path = std::env::current_exe().ok().map(|p| p.with_file_name("cpuzeez.ini"));
        let text = path.and_then(|p| std::fs::read(p).ok()).map(decode).unwrap_or_default();
        Config::parse(&text)
    }

    pub fn parse(text: &str) -> Config {
        let mut c = Config::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            let (k, v) = (k.trim().to_ascii_lowercase(), v.trim());
            let on = !(v == "0" || v.eq_ignore_ascii_case("off"));
            let num = || v.parse::<f32>().unwrap_or(0.0);
            let rgb = || u32::from_str_radix(v, 16).unwrap_or(0);
            match k.as_str() {
                "acpi" => c.acpi = on,
                "pci" => c.pci = on,
                "maxpcibus" => c.max_pci_bus = v.parse().unwrap_or(256),
                "dmi" => c.dmi = on,
                "sensor" => c.sensor = on,
                "smbus" => c.smbus = on,
                "display" => c.display = on,
                "usedisplayapi" => c.use_display_api = on,
                "busclock" => c.bus_clock = on,
                "chipset" => c.chipset = on,
                "spd" => c.spd = on,
                "lpcio" => c.lpcio = on,
                "smart" => c.smart = on,
                "xoc" => c.xoc = on,
                "checkupdates" => c.check_updates = on,
                "textfontname" => c.text_font_name = v.into(),
                "textfontsize" => c.text_font_size = num(),
                "textfontcolor" => c.text_font_color = rgb(),
                "labelfontname" => c.label_font_name = v.into(),
                "labelfontsize" => c.label_font_size = num(),
                "labelfontcolor" => c.label_font_color = rgb(),
                _ => {}
            }
        }
        // PCI off disables chipset, SPD and SMBus too (cpuz_readme.txt).
        if !c.pci {
            c.chipset = false;
            c.spd = false;
            c.smbus = false;
        }
        c
    }
}

/// cpuz.ini ships as UTF-16LE with BOM; accept that or UTF-8.
fn decode(b: Vec<u8>) -> String {
    if b.starts_with(&[0xFF, 0xFE]) {
        let u: Vec<u16> = b[2..].as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        String::from_utf16_lossy(&u)
    } else {
        String::from_utf8_lossy(&b).into_owned()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn parse() {
        let c = super::Config::parse("[CPU-Z]\nSPD=0\nMaxPCIBus=64\nTextFontColor=0000A0\nPCI=1");
        assert!(!c.spd && c.pci && c.max_pci_bus == 64 && c.text_font_color == 0xA0);
        assert!(!super::Config::parse("PCI=off").chipset);
    }
}
