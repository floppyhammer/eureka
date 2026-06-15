use font_kit::source::SystemSource;

pub fn find_system_font(font_name: &str) -> Option<Vec<u8>> {
    let result = std::panic::catch_unwind(|| {
        let mut font = None;
        let source = SystemSource::new();

        if !font_name.is_empty() {
            let res = source.select_by_postscript_name(font_name);

            if res.is_ok() {
                font = Some(res.unwrap().load().unwrap());
            }
        }

        if font.is_none() {
            let family_names = [font_kit::family_name::FamilyName::Serif];
            let properties = font_kit::properties::Properties::default();

            let res = source.select_best_match(&family_names, &properties);

            if res.is_ok() {
                font = Some(res.unwrap().load().unwrap());
            }
        }

        if font.is_none() {
            let handle = source.all_fonts().unwrap().first().unwrap().clone();

            font = Some(handle.load().unwrap());
        }

        let font_data = font
            .take()
            .expect("Font fallback failed!")
            .copy_font_data()
            .unwrap();
        let font_data = (*font_data).clone();

        Some(font_data)
    });
    if result.is_err() {
        eprintln!("ERROR: failed to find font: {}", font_name);
        return None;
    }

    result.unwrap()
}
