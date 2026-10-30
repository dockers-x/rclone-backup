import { TOY_PLASTIC, ToyKey } from "@/components/shared/toy";
import type { ToastKind, ToastWorldProps } from "../toast";

const PLASTIC: Record<
  ToastKind,
  (typeof TOY_PLASTIC)[keyof typeof TOY_PLASTIC]
> = {
  success: TOY_PLASTIC.green,
  error: TOY_PLASTIC.red,
  info: TOY_PLASTIC.blue,
};

const ICON: Record<ToastKind, string> = { success: "✓", error: "!", info: "i" };

export default function ToyToast({
  kind,
  title,
  description,
  action,
  dismiss,
}: ToastWorldProps) {
  const [face, side, ink] = PLASTIC[kind];
  return (
    <div
      data-ovio-world="toy"
      className="flex w-full flex-wrap items-center gap-3 rounded-(--ovio-radius) bg-(--ovio-surface) p-3 font-(family-name:--ovio-font) text-(--ovio-ink) shadow-[inset_0_1px_0_#fff,0_6px_0_#d2ccbf,0_18px_24px_-12px_rgba(40,28,10,.45)]"
    >
      <span
        aria-hidden
        className="flex size-9 shrink-0 items-center justify-center rounded-full text-lg font-extrabold"
        style={{ background: face, color: ink, boxShadow: `0 4px 0 ${side}` }}
      >
        {ICON[kind]}
      </span>
      <div className="min-w-40 flex-1 wrap-anywhere">
        <p className="m-0 text-sm leading-snug font-extrabold">{title}</p>
        {description && (
          <p className="m-0 mt-0.5 font-(family-name:--ovio-mono) text-[11.5px] text-(--ovio-muted)">
            {description}
          </p>
        )}
      </div>
      {action && (
        <ToyKey
          depth={4}
          side={side}
          onClick={action.onClick}
          className="shrink-0 cursor-pointer rounded-[10px] border-0 px-3 py-1.5 font-(family-name:--ovio-font) text-xs font-extrabold"
          style={{ background: face, color: ink }}
        >
          {action.label}
        </ToyKey>
      )}
      <ToyKey
        depth={3}
        side="#cfc8b8"
        aria-label={
          typeof document !== "undefined" &&
          document.documentElement.lang.startsWith("zh")
            ? "关闭通知"
            : "Dismiss"
        }
        onClick={dismiss}
        className="size-7 shrink-0 cursor-pointer rounded-full border-0 bg-white text-sm font-extrabold text-(--ovio-ink)"
      >
        ×
      </ToyKey>
    </div>
  );
}
