<script lang="ts">
  import SearchableSelect from "$lib/components/SearchableSelect.svelte";
  import { languages, getLanguageSearchTerms } from "$lib/config/languages";
  import { t } from "$lib/i18n";
  let { value, onchange, autoDetect = false, placeholder, className = "", disabled = false, placement = "down", noResultsText = t("common.noResults") } = $props<{
    value: string;
    onchange: (value: string) => void;
    autoDetect?: boolean;
    placeholder?: string;
    className?: string;
    disabled?: boolean;
    placement?: "up" | "down";
    noResultsText?: string;
  }>();
  let options = $derived([
    ...(autoDetect ? [{ value: "auto", label: t("transcribe.autoDetect"), icon: "🌐", searchTerms: "auto detect" }] : []),
    ...languages.map(language => ({
      value: language.code,
      label: language.nameEn === language.name ? language.name : `${language.nameEn} — ${language.name}`,
      icon: language.flag,
      searchTerms: getLanguageSearchTerms(language.code),
    })),
  ]);
</script>
<SearchableSelect {options} {value} {onchange} {placeholder} {className} {disabled} {placement} {noResultsText} />
