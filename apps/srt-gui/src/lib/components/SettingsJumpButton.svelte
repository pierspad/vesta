<script lang="ts">
  type SettingsSection = "overview" | "llm" | "whisper" | "language" | "anki" | "shortcuts";

  interface Props {
    label: string;
    section?: SettingsSection;
    targetId?: string;
    onjump?: (section: SettingsSection, targetId?: string) => void;
    onclick?: () => void;
    disabled?: boolean;
    class?: string;
    variant?: "default" | "amber" | "cyan" | "subtle";
  }

  let {
    label,
    section = "whisper",
    targetId,
    onjump,
    onclick,
    disabled = false,
    class: classProp = "",
    variant = "amber",
  }: Props = $props();

  function handleClick() {
    if (disabled) return;
    if (onclick) {
      onclick();
    } else if (onjump) {
      onjump(section, targetId);
    }
  }

  const variantClasses: Record<string, string> = {
    amber: "text-amber-300 bg-amber-500/10 hover:bg-amber-500/20 border-amber-500/30 hover:border-amber-400/50 shadow-amber-500/5",
    cyan: "text-cyan-300 bg-cyan-500/10 hover:bg-cyan-500/20 border-cyan-500/30 hover:border-cyan-400/50 shadow-cyan-500/5",
    subtle: "text-gray-300 hover:text-white bg-white/5 hover:bg-white/10 border-white/10 hover:border-white/20",
    default: "text-amber-300 bg-amber-500/10 hover:bg-amber-500/20 border-amber-500/30 hover:border-amber-400/50 shadow-amber-500/5",
  };
</script>

<button
  type="button"
  {disabled}
  onclick={handleClick}
  class="group inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold border transition-all duration-200 cursor-pointer disabled:cursor-not-allowed disabled:opacity-45 shadow-sm {variantClasses[variant] ?? variantClasses.default} {classProp}"
>
  <svg class="w-3.5 h-3.5 text-current opacity-80 group-hover:rotate-45 transition-transform shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
  </svg>
  <span class="truncate">{label}</span>
  <svg class="w-3 h-3 text-current opacity-70 group-hover:translate-x-0.5 transition-transform shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
  </svg>
</button>
