/// Informazioni sulla lingua target per la traduzione
pub struct LanguageInfo {
    pub full_name: &'static str,
    pub examples: &'static str,
}

/// Mappa i codici lingua ISO ai nomi completi e agli esempi few-shot.
///
/// Un `match` su un numero fisso di lingue note è risolto dal compilatore
/// come salto diretto: niente allocazione di `HashMap` né hashing a runtime
/// ad ogni chiamata, a differenza della tabella costruita dinamicamente in
/// precedenza.
pub fn get_language_info(lang_code: &str) -> LanguageInfo {
    let (full_name, examples) = match lang_code {
        "it" => ("Italian",
                r#"Examples of high-quality subtitle translations to Italian:

                English: "I can't believe this is happening."
                Italian: "Non posso credere che stia succedendo."

                English: "What are you doing here?"
                Italian: "Cosa ci fai qui?"

                English: "Let's get out of here,
                before it's too late."
                Italian: "Usciamo da qui,
                prima che sia troppo tardi."

                English: "I'm through doing that shit."
                Italian: "Ho chiuso con queste stronzate."
                "#),

        "es" => ("Spanish",
                r#"Examples of high-quality subtitle translations to Spanish:

                English: "I can't believe this is happening."
                Spanish: "No puedo creer que esto esté pasando."

                English: "What are you doing here?"
                Spanish: "¿Qué haces aquí?"

                English: "Let's get out of here,
                before it's too late."
                Spanish: "Salgamos de aquí,
                antes de que sea demasiado tarde."

                English: "I'm through doing that shit."
                Spanish: "He terminado con esa mierda."
                "#),

        "fr" => ("French",
                r#"Examples of high-quality subtitle translations to French:

                English: "I can't believe this is happening."
                French: "Je n'arrive pas à croire que ça arrive."

                English: "What are you doing here?"
                French: "Qu'est-ce que tu fais ici ?"

                English: "Let's get out of here,
                before it's too late."
                French: "Sortons d'ici,
                avant qu'il ne soit trop tard."

                English: "I'm through doing that shit."
                French: "J'en ai fini avec ces conneries."
                "#),

        "de" => ("German",
                r#"Examples of high-quality subtitle translations to German:

                English: "I can't believe this is happening."
                German: "Ich kann nicht glauben, dass das passiert."

                English: "What are you doing here?"
                German: "Was machst du hier?"

                English: "Let's get out of here,
                before it's too late."
                German: "Lass uns hier verschwinden,
                bevor es zu spät ist."

                English: "I'm through doing that shit."
                German: "Ich habe mit diesem Scheiß abgeschlossen."
                "#),

        "en" => ("English",
                r#"Examples of high-quality subtitle translations to English:

                Italian: "Non posso credere che stia succedendo."
                English: "I can't believe this is happening."

                Italian: "Cosa ci fai qui?"
                English: "What are you doing here?"

                Italian: "Usciamo da qui,
                prima che sia troppo tardi."
                English: "Let's get out of here,
                before it's too late."

                Italian: "Ho chiuso con queste stronzate."
                English: "I'm through doing that shit."
                "#),

        "pt" => ("Portuguese",
                r#"Examples of high-quality subtitle translations to Portuguese:

                English: "I can't believe this is happening."
                Portuguese: "Não posso acreditar que isso está acontecendo."

                English: "What are you doing here?"
                Portuguese: "O que você está fazendo aqui?"

                English: "Let's get out of here,
                before it's too late."
                Portuguese: "Vamos sair daqui,
                antes que seja tarde demais."

                English: "I'm through doing that shit."
                Portuguese: "Cansei dessa merda."
                "#),

        "ru" => ("Russian",
                r#"Examples of high-quality subtitle translations to Russian:

                English: "I can't believe this is happening."
                Russian: "Не могу поверить, что это происходит."

                English: "What are you doing here?"
                Russian: "Что ты здесь делаешь?"

                English: "Let's get out of here,
                before it's too late."
                Russian: "Давай уйдём отсюда,
                пока не слишком поздно."

                English: "I'm through doing that shit."
                Russian: "С меня хватит этого дерьма."
                "#),

        "ja" => ("Japanese",
                r#"Examples of high-quality subtitle translations to Japanese:

                English: "I can't believe this is happening."
                Japanese: "信じられない。"

                English: "What are you doing here?"
                Japanese: "ここで何してるの？"

                English: "Let's get out of here,
                before it's too late."
                Japanese: "ここから出よう、
                手遅れになる前に。"

                English: "I'm through doing that shit."
                Japanese: "もううんざりだ。"
                "#),

        "zh" => ("Chinese (Simplified)",
                r#"Examples of high-quality subtitle translations to Chinese (Simplified):

                English: "I can't believe this is happening."
                Chinese: "我不敢相信这正在发生。"

                English: "What are you doing here?"
                Chinese: "你在这里做什么？"

                English: "Let's get out of here,
                before it's too late."
                Chinese: "我们离开这里吧,
                趁还来得及。"

                English: "I'm through doing that shit."
                Chinese: "我受够这破事了。"
                "#),

        "ar" => ("Arabic",
                r#"Examples of high-quality subtitle translations to Arabic:

                English: "I can't believe this is happening."
                Arabic: "لا أصدق أن هذا يحدث."

                English: "What are you doing here?"
                Arabic: "ماذا تفعل هنا؟"

                English: "Let's get out of here,
                before it's too late."
                Arabic: "لنخرج من هنا،
                قبل فوات الأوان."

                English: "I'm through doing that shit."
                Arabic: "لقد انتهيت من هذا الهراء."
                "#),

        _ => ("the target language", ""),
    };

    LanguageInfo { full_name, examples }
}
