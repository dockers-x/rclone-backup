import {
  Children,
  cloneElement,
  isValidElement,
  useEffect,
  useId,
  useRef,
  useState,
} from "react";
import * as Dialog from "@radix-ui/react-dialog";
import * as AlertDialog from "@radix-ui/react-alert-dialog";
import * as Switch from "@radix-ui/react-switch";
import { X, LoaderCircle, FolderOpen } from "lucide-react";
import { ToyKey } from "./shared/toy";
import { useWorld } from "./shared/world-provider";
import { LiquidTabs } from "./ovio/liquid-tabs/liquid-tabs";

export function Button({
  children,
  variant = "secondary",
  busy = false,
  className = "",
  ...props
}) {
  const world = useWorld();
  const Component = world === "toy" ? ToyKey : "button";
  return (
    <Component
      type="button"
      {...(world === "toy" ? { depth: 3, side: "var(--line-strong)" } : {})}
      className={`button ${variant} ${className}`}
      {...props}
      disabled={props.disabled || busy}
      aria-busy={busy || undefined}
    >
      {busy && <LoaderCircle className="spin" size={16} aria-hidden="true" />}
      {children}
    </Component>
  );
}
export function IconButton({ icon: Icon, label, ...props }) {
  return (
    <Button className="icon-button" aria-label={label} title={label} {...props}>
      <Icon size={18} aria-hidden="true" />
    </Button>
  );
}
export function Field({ label, hint, children, className = "" }) {
  const id = useId();
  const associate = (nodes) =>
    Children.map(nodes, (node) => {
      if (!isValidElement(node)) return node;
      if (["input", "select", "textarea"].includes(node.type))
        return cloneElement(node, {
          id,
          "aria-describedby": hint ? `${id}-hint` : undefined,
        });
      return node.props.children
        ? cloneElement(node, {}, associate(node.props.children))
        : node;
    });
  return (
    <div className={`field ${className}`}>
      <label htmlFor={id}>{label}</label>
      {associate(children)}
      {hint && <small id={`${id}-hint`}>{hint}</small>}
    </div>
  );
}
export function Toggle({ checked, onChange, label, hint }) {
  return (
    <label className="toggle-field">
      <span>
        <strong>{label}</strong>
        {hint && <small>{hint}</small>}
      </span>
      <Switch.Root
        checked={checked}
        onCheckedChange={onChange}
        className="switch"
        aria-label={label}
      >
        <Switch.Thumb className="switch-thumb" />
      </Switch.Root>
    </label>
  );
}
export function Badge({ status, children }) {
  return (
    <span className={`badge ${status || ""}`}>
      <span aria-hidden="true" />
      {children}
    </span>
  );
}
export function Empty({ icon: Icon = FolderOpen, title, hint, children }) {
  return (
    <div className="empty-state">
      <div className="empty-glyph">
        <Icon size={28} strokeWidth={1.4} />
      </div>
      <h3>{title}</h3>
      {hint && <p>{hint}</p>}
      {children}
    </div>
  );
}
export function ErrorNotice({ children }) {
  return children ? (
    <div className="error-notice" role="alert">
      {children}
    </div>
  ) : null;
}
export function Loading() {
  return (
    <div className="loading" role="status">
      <LoaderCircle className="spin" size={20} />
      <span>
        {document.documentElement.lang.startsWith("zh")
          ? "正在加载…"
          : "Loading…"}
      </span>
    </div>
  );
}
export function Tabs({ tabs, value, onChange, label, panels }) {
  return (
    <LiquidTabs
      tabs={tabs}
      value={value}
      onValueChange={onChange}
      label={label}
      panels={panels}
      className="app-tabs"
    />
  );
}
// These dialogs open from application state rather than Radix Trigger nodes.
// Capture the opener before portal inputs with autoFocus mount.
export function useDialogFocus() {
  const opener = useRef(document.activeElement);
  return {
    onCloseAutoFocus(event) {
      event.preventDefault();
      const target = opener.current;
      if (
        target instanceof HTMLElement &&
        target.isConnected &&
        !target.closest("[hidden]")
      ) {
        target.focus();
      } else {
        document.querySelector("main h1")?.focus();
      }
    },
  };
}
export function Modal({
  title,
  description,
  children,
  onClose,
  wide = false,
  footer,
  closeLabel = "Close",
}) {
  const focus = useDialogFocus();
  return (
    <Dialog.Root
      open
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay className="modal-overlay" />
        <Dialog.Content
          {...focus}
          className={`modal-content ${wide ? "wide" : ""}`}
          onPointerDownOutside={(e) => e.preventDefault()}
        >
          <header className="modal-header">
            <div>
              <Dialog.Title>{title}</Dialog.Title>
              {description ? (
                <Dialog.Description>{description}</Dialog.Description>
              ) : (
                <Dialog.Description className="sr-only">
                  {title}
                </Dialog.Description>
              )}
            </div>
            <Dialog.Close asChild>
              <IconButton icon={X} label={closeLabel} />
            </Dialog.Close>
          </header>
          {children}
          {footer}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
export function Confirm({ title, description, onConfirm, onClose, t }) {
  const focus = useDialogFocus();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  return (
    <AlertDialog.Root
      open
      onOpenChange={(open) => {
        if (!open && !busy) onClose();
      }}
    >
      <AlertDialog.Portal>
        <AlertDialog.Overlay className="modal-overlay" />
        <AlertDialog.Content
          {...focus}
          className="modal-content confirm-dialog"
        >
          <AlertDialog.Title>{title}</AlertDialog.Title>
          <AlertDialog.Description>{description}</AlertDialog.Description>
          <ErrorNotice>{error}</ErrorNotice>
          <div className="modal-footer">
            <AlertDialog.Cancel asChild>
              <Button disabled={busy}>{t("cancel")}</Button>
            </AlertDialog.Cancel>
            <Button
              variant="danger"
              busy={busy}
              onClick={async () => {
                setBusy(true);
                try {
                  await onConfirm();
                  onClose();
                } catch (e) {
                  setError(e.message);
                } finally {
                  setBusy(false);
                }
              }}
            >
              {t("delete")}
            </Button>
          </div>
        </AlertDialog.Content>
      </AlertDialog.Portal>
    </AlertDialog.Root>
  );
}
export function useNow() {
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, []);
  return now;
}
