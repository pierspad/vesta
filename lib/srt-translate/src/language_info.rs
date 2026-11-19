pub struct LanguageInfo {
    pub full_name: &'static str,
    pub examples: &'static str,
}

pub fn get_language_info(lang_code: &str) -> LanguageInfo {
    let normalized = lang_code.trim().to_ascii_lowercase();
    let (full_name, examples) = match normalized.as_str() {
        "it" | "italian" => (
            "Italian",
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
                "#,
        ),

        "es" | "spanish" => (
            "Spanish",
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
                "#,
        ),

        "fr" | "french" => (
            "French",
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
                "#,
        ),

        "de" | "german" => (
            "German",
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
                "#,
        ),

        "en" | "english" => (
            "English",
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
                "#,
        ),

        "pt" | "portuguese" | "pt-br" => (
            "Portuguese",
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
                "#,
        ),

        "ru" | "russian" => (
            "Russian",
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
                "#,
        ),

        "ja" | "japanese" => (
            "Japanese",
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
                "#,
        ),

        "zh" | "chinese" | "zh-cn" | "zh-tw" => (
            "Chinese (Simplified)",
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
                "#,
        ),

        "ar" | "arabic" => (
            "Arabic",
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
                "#,
        ),

        "ko" | "korean" => (
            "Korean",
            r#"Examples of high-quality subtitle translations to Korean:

                English: "I can't believe this is happening."
                Korean: "이런 일이 벌어지다니 믿을 수 없어."

                English: "What are you doing here?"
                Korean: "여기서 뭐 하는 거야?"

                English: "Let's get out of here,
                before it's too late."
                Korean: "늦기 전에
                여기서 나가자."

                English: "I'm through doing that shit."
                Korean: "그딴 짓은 이제 지긋지긋해."
                "#,
        ),

        "nl" | "dutch" => (
            "Dutch",
            r#"Examples of high-quality subtitle translations to Dutch:

                English: "I can't believe this is happening."
                Dutch: "Ik kan niet geloven dat dit gebeurt."

                English: "What are you doing here?"
                Dutch: "Wat doe je hier?"

                English: "Let's get out of here,
                before it's too late."
                Dutch: "Laten we hier weggaan,
                voordat het te laat is."

                English: "I'm through doing that shit."
                Dutch: "Ik ben klaar met die rotzooi."
                "#,
        ),

        "pl" | "polish" => (
            "Polish",
            r#"Examples of high-quality subtitle translations to Polish:

                English: "I can't believe this is happening."
                Polish: "Nie mogę uwierzyć, że to się dzieje."

                English: "What are you doing here?"
                Polish: "Co tu robisz?"

                English: "Let's get out of here,
                before it's too late."
                Polish: "Wynikajmy stąd,
                zanim będzie za późno."

                English: "I'm through doing that shit."
                Polish: "Mam dość tego gówna."
                "#,
        ),

        "tr" | "turkish" => (
            "Turkish",
            r#"Examples of high-quality subtitle translations to Turkish:

                English: "I can't believe this is happening."
                Turkish: "Bunun olduğuna inanamıyorum."

                English: "What are you doing here?"
                Turkish: "Burada ne yapıyorsun?"

                English: "Let's get out of here,
                before it's too late."
                Turkish: "Çok geç olmadan
                buradan gidelim."

                English: "I'm through doing that shit."
                Turkish: "Bu saçmalıktan bıktım artık."
                "#,
        ),

        "hi" | "hindi" => (
            "Hindi",
            r#"Examples of high-quality subtitle translations to Hindi:

                English: "I can't believe this is happening."
                Hindi: "मुझे यकीन नहीं हो रहा कि यह हो रहा है।"

                English: "What are you doing here?"
                Hindi: "तुम यहाँ क्या कर रहे हो?"

                English: "Let's get out of here,
                before it's too late."
                Hindi: "इससे पहले कि बहुत देर हो जाए,
                यहाँ से निकल चलते हैं।"

                English: "I'm through doing that shit."
                Hindi: "मैं इस बकवास से तंग आ चुका हूँ।"
                "#,
        ),

        _ => ("the target language", ""),
    };

    LanguageInfo {
        full_name,
        examples,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_case_insensitive_codes_full_names_and_all_locales() {
        for (input, expected) in [
            ("IT", "Italian"),
            (" Italian ", "Italian"),
            ("es", "Spanish"),
            ("French", "French"),
            ("DE", "German"),
            ("English", "English"),
            ("pt-br", "Portuguese"),
            ("Russian", "Russian"),
            ("ja", "Japanese"),
            ("zh-cn", "Chinese (Simplified)"),
            ("ar", "Arabic"),
            ("ko", "Korean"),
            ("Korean", "Korean"),
            ("nl", "Dutch"),
            ("pl", "Polish"),
            ("tr", "Turkish"),
            ("hi", "Hindi"),
        ] {
            let info = get_language_info(input);
            assert_eq!(info.full_name, expected, "input={input}");
            assert!(!info.examples.is_empty(), "missing examples for {input}");
        }
        assert_eq!(
            get_language_info("klingon").full_name,
            "the target language"
        );
    }
}
