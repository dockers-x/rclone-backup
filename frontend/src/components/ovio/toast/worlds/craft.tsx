import type { ToastKind, ToastWorldProps } from "../toast";

const STAMP: Record<ToastKind, { text: string; color: string }> = {
  success: { text: "Done", color: "#2f7a4a" },
  error: { text: "Oops", color: "var(--ovio-accent-deep)" },
  info: { text: "Note", color: "#2b4a9b" },
};

export default function CraftToast({
  kind,
  title,
  description,
  action,
  dismiss,
}: ToastWorldProps) {
  const stamp = STAMP[kind];
  return (
    <div
      data-ovio-world="craft"
      className="relative w-full -rotate-1 rounded-sm bg-(--ovio-surface) px-4 pt-4 pb-3.5 font-(family-name:--ovio-font) text-(--ovio-ink) shadow-[0_2px_4px_rgba(70,45,20,.12),0_16px_26px_-14px_rgba(70,45,20,.45)]"
    >
      <span
        aria-hidden
        className="absolute -top-2.5 left-6 h-5 w-16 -rotate-3 bg-(--ovio-tape)"
      />
      <div className="flex items-start gap-2.5">
        <span
          aria-hidden
          className="mt-0.5 shrink-0 -rotate-6 rounded border-2 px-1.5 text-[11px] leading-[1.4] font-extrabold tracking-[0.04em] uppercase"
          style={{ color: stamp.color, borderColor: stamp.color }}
        >
          {typeof document !== "undefined" &&
          document.documentElement.lang.startsWith("zh")
            ? { success: "完成", error: "出错", info: "提示" }[kind]
            : stamp.text}
        </span>
        <p className="m-0 min-w-0 flex-1 wrap-anywhere text-[15px] leading-snug font-bold">
          {title}
        </p>
        <button
          type="button"
          aria-label={
            typeof document !== "undefined" &&
            document.documentElement.lang.startsWith("zh")
              ? "关闭通知"
              : "Dismiss"
          }
          onClick={dismiss}
          className="-mt-1 size-6 shrink-0 cursor-pointer border-0 bg-transparent text-lg leading-none text-(--ovio-muted)"
        >
          ×
        </button>
      </div>
      {description && (
        <p className="m-0 mt-1 font-(family-name:--ovio-hand) text-[19px] leading-tight text-[#2b4a9b]">
          {description}
        </p>
      )}
      {action && (
        <button
          type="button"
          onClick={action.onClick}
          className="mt-2.5 -rotate-2 cursor-pointer border-0 bg-[#ffe27a] px-2.5 py-1 font-(family-name:--ovio-mono) text-[11px] text-(--ovio-ink) shadow-[0_3px_6px_-2px_rgba(70,45,20,.4)] transition-[rotate] duration-300 ease-[cubic-bezier(.34,1.5,.5,1)] motion-reduce:transition-none hover:rotate-0"
        >
          {action.label}
        </button>
      )}
    </div>
  );
}
