use super::super::types::FlashcardConfig;
use super::{ANKI_BACK_TEMPLATE, ANKI_CARD_STYLING, ANKI_FRONT_TEMPLATE};

fn clean_field_name(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn push_model_field(field_defs: &mut Vec<String>, ord: &mut i32, name: &str) {
    let json_name = serde_json::to_string(name).unwrap_or_else(|_| "\"Field\"".to_string());
    field_defs.push(format!(
        r#"{{"name":{},"ord":{},"sticky":false,"rtl":false,"font":"Arial","size":20,"description":"","plainText":false,"collapsed":false,"excludeFromSearch":false}}"#,
        json_name, ord
    ));
    *ord += 1;
}

fn anki_field_ref(name: &str) -> String {
    format!("{{{{{}}}}}", name)
}

fn rewrite_template_field_tokens(template: &str, replacements: &[(&str, &str)]) -> String {
    let mut output = template.to_string();

    for (index, (canonical, _)) in replacements.iter().enumerate() {
        output = output.replace(
            &anki_field_ref(canonical),
            &format!("__vesta_FIELD_TOKEN_{index}__"),
        );
    }

    for (index, (_, actual)) in replacements.iter().enumerate() {
        output = output.replace(
            &format!("__vesta_FIELD_TOKEN_{index}__"),
            &anki_field_ref(actual),
        );
    }

    output
}

pub(crate) fn build_collection_metadata(
    config: &FlashcardConfig,
    timestamp: i64,
    deck_id: i64,
    model_id: i64,
) -> (String, String, String, String) {
    let note_type_name = config.note_type_name.as_deref().unwrap_or("subs2srs");
    let field_names = config.field_names.clone().unwrap_or_default();
    let names = [
        clean_field_name(&field_names.expression, "Expression"),
        clean_field_name(&field_names.meaning, "Meaning"),
        clean_field_name(&field_names.reading, "Reading"),
        clean_field_name(&field_names.audio, "Audio"),
        clean_field_name(&field_names.snapshot, "Snapshot"),
        clean_field_name(&field_names.video, "Video"),
        clean_field_name(&field_names.tags, "Tags"),
        clean_field_name(&field_names.sequence_marker, "SequenceMarker"),
        clean_field_name(&field_names.notes, "Notes"),
    ];
    let of = &config.output_fields;
    let schema = [
        (of.include_subs1, names[0].as_str()),
        (of.include_subs2, names[1].as_str()),
        (of.include_audio, names[3].as_str()),
        (of.include_snapshot, names[4].as_str()),
        (of.include_video, names[5].as_str()),
        (of.include_tag, names[6].as_str()),
        (of.include_sequence, names[7].as_str()),
        (of.include_reading, names[2].as_str()),
        (of.include_notes, names[8].as_str()),
    ];
    let mut field_defs = Vec::new();
    let mut ord = 0;
    for (included, name) in schema {
        if included {
            push_model_field(&mut field_defs, &mut ord, name);
        }
    }
    if field_defs.is_empty() {
        push_model_field(&mut field_defs, &mut ord, &names[0]);
    }

    let replacements = [
        ("Expression", names[0].as_str()),
        ("Meaning", names[1].as_str()),
        ("Reading", names[2].as_str()),
        ("Audio", names[3].as_str()),
        ("Snapshot", names[4].as_str()),
        ("Video", names[5].as_str()),
        ("Tags", names[6].as_str()),
        ("Tag", names[6].as_str()),
        ("SequenceMarker", names[7].as_str()),
        ("Notes", names[8].as_str()),
    ];
    let mut qfmt = rewrite_template_field_tokens(
        config
            .card_front_html
            .as_deref()
            .unwrap_or(ANKI_FRONT_TEMPLATE),
        &replacements,
    );
    let mut afmt = rewrite_template_field_tokens(
        config
            .card_back_html
            .as_deref()
            .unwrap_or(ANKI_BACK_TEMPLATE),
        &replacements,
    );
    if config.dictionary_lookup {
        let script = super::dictionary::lookup_script(config.target_language.as_deref());
        qfmt.push_str(&script);
        afmt.push_str(&script);
    }
    let css = crate::fonts::maybe_prepend_font_vars(
        config.card_css.as_deref().unwrap_or(ANKI_CARD_STYLING),
        config,
    );
    let note_type_json =
        serde_json::to_string(note_type_name).unwrap_or_else(|_| "\"subs2srs\"".to_string());
    let qfmt_json = serde_json::to_string(&qfmt).unwrap_or_else(|_| "\"\"".to_string());
    let afmt_json = serde_json::to_string(&afmt).unwrap_or_else(|_| "\"\"".to_string());
    let css_json = serde_json::to_string(&css).unwrap_or_else(|_| "\"\"".to_string());
    let models = format!(
        r#"{{"{model_id}":{{"id":{model_id},"name":{note_type_json},"type":0,"mod":{timestamp},"usn":-1,"sortf":0,"did":{deck_id},"tmpls":[{{"name":"Card 1","ord":0,"qfmt":{qfmt_json},"afmt":{afmt_json},"did":null,"bqfmt":"","bafmt":""}}],"flds":[{fields}],"css":{css_json},"latexPre":"\\\\documentclass[12pt]{{article}}\\\\special{{papersize=3in,5in}}\\\\usepackage[utf8]{{inputenc}}\\\\usepackage{{amssymb,amsmath}}\\\\pagestyle{{empty}}\\\\setlength{{\\\\parindent}}{{0in}}\\\\begin{{document}}\\n","latexPost":"\\\\end{{document}}","latexsvg":false,"req":[[0,"all",[0]]]}}}}"#,
        fields = field_defs.join(","),
    );
    let deck_name_json =
        serde_json::to_string(&config.deck_name).unwrap_or_else(|_| "\"Vesta\"".to_string());
    let decks = format!(
        r#"{{"{deck_id}":{{"id":{deck_id},"name":{deck_name_json},"mod":{timestamp},"usn":-1,"lrnToday":[0,0],"revToday":[0,0],"newToday":[0,0],"timeToday":[0,0],"collapsed":false,"browserCollapsed":false,"desc":"","dyn":0,"conf":1,"extendNew":10,"extendRev":50}}}}"#
    );
    let dconf = r#"{"1":{"id":1,"name":"Default","replayq":true,"lapse":{"delays":[10],"mult":0,"minInt":1,"leechFails":8,"leechAction":0},"rev":{"perDay":200,"ease4":1.3,"fuzz":0.05,"minSpace":1,"ivlFct":1,"maxIvl":36500,"buried":false,"hardFactor":1.2},"timer":0,"maxTaken":60,"usn":0,"new":{"delays":[1,10],"ints":[1,4,0],"initialFactor":2500,"order":1,"perDay":20,"buried":false},"mod":0,"autoplay":true}}"#.to_string();
    let conf = r#"{"activeDecks":[1],"curDeck":1,"newSpread":0,"collapseTime":1200,"timeLim":0,"estTimes":true,"dueCounts":true,"curModel":null,"nextPos":1,"sortType":"noteFld","sortBackwards":false,"addToCur":true}"#.to_string();
    (conf, models, decks, dconf)
}
