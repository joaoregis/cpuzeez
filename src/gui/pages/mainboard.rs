//! Mainboard page.

#[allow(unused_imports)]
use crate::gui::*;

pub(crate) fn mainboard_tab(ui: &mut egui::Ui, r: &[Section]) {
    boxed(
        ui,
        "Motherboard",
        &[
            ("Manufacturer", sgi(r, "DMI", "DMI Baseboard", "vendor")),
            ("Model", sgi(r, "DMI", "DMI Baseboard", "model")),
            ("Revision", sgi(r, "DMI", "DMI Baseboard", "revision")),
            ("Bus Specs.", sget(r, "Chipset", "Bus Specification")),
            ("Chipset", sget(r, "Chipset", "Northbridge")),
            ("Southbridge", sget(r, "Chipset", "Southbridge")),
            (
                "LPCIO",
                format!("{} {}", sget(r, "LPCIO", "LPCIO Vendor"), sget(r, "LPCIO", "LPCIO Model")).trim().to_string(),
            ),
        ],
    );
    boxed(
        ui,
        "BIOS",
        &[
            ("Brand", sgi(r, "DMI", "DMI BIOS", "vendor")),
            ("Version", sgi(r, "DMI", "DMI BIOS", "version")),
            ("Date", sgi(r, "DMI", "DMI BIOS", "date")),
            ("UEFI", sget(r, "BIOS", "UEFI")),
            ("Microcode", sget(r, "Processors Information", "Microcode Revision")),
        ],
    );
    boxed(
        ui,
        "Graphic Interface",
        &[
            ("Bus", sget(r, "Chipset", "Graphic Interface")),
            ("Current Link Width", sget(r, "Chipset", "PCI-E Link Width")),
            ("Current Link Speed", sget(r, "Chipset", "PCI-E Link Speed")),
        ],
    );
}
