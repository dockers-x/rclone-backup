"use client";

import { useSyncExternalStore, type ComponentType } from "react";
import { createPortal } from "react-dom";
import { toast, Toaster, type ExternalToast, type ToasterProps } from "sonner";
import { OvioProvider, useWorld } from "@/components/shared/world-provider";
import { everyWorld, type InstalledWorld } from "@/lib/world";
import type { ToastKind, ToastAction } from "./types";
import * as worlds from "./worlds";

export type { ToastKind, ToastAction } from "./types";

export type OvioToastOptions = Pick<
  ExternalToast,
  "id" | "duration" | "toasterId"
> & {
  description?: string;
  /** A button on the toast; pressing it also dismisses the toast. */
  action?: ToastAction;
};

/** Everything a world needs to draw one toast. Sonner owns stacking, timing and swipe. */
export type ToastWorldProps = {
  kind: ToastKind;
  title: string;
  description?: string;
  action?: ToastAction;
  dismiss: () => void;
};

/** The installed worlds; a single-world install draws its one world for any provider world. */
const VIEWS = everyWorld<ComponentType<ToastWorldProps>>(worlds);

function OvioToast(props: ToastWorldProps) {
  const View = VIEWS[useWorld()] ?? VIEWS.minimal;
  return <View {...props} />;
}

function show(kind: ToastKind, title: string, options: OvioToastOptions = {}) {
  const { description, action, ...rest } = options;
  return toast.custom(
    (id) => (
      <OvioToast
        kind={kind}
        title={title}
        description={description}
        action={
          action && {
            label: action.label,
            onClick: () => {
              toast.dismiss(id);
              action.onClick();
            },
          }
        }
        dismiss={() => toast.dismiss(id)}
      />
    ),
    { duration: kind === "error" ? 8000 : 4000, ...rest },
  );
}

/** Shows a toast in the world of the OvioToaster that renders it. Returns its id. */
export const ovioToast = {
  success: (title: string, options?: OvioToastOptions) =>
    show("success", title, options),
  error: (title: string, options?: OvioToastOptions) =>
    show("error", title, options),
  info: (title: string, options?: OvioToastOptions) =>
    show("info", title, options),
  dismiss: (id?: string | number) => toast.dismiss(id),
};

export type OvioToasterProps = Omit<
  ToasterProps,
  "theme" | "richColors" | "invert" | "icons" | "toastOptions" | "closeButton"
> & { variant?: InstalledWorld<typeof worlds> };

/** Where toasts appear. Mount it once; toasts take its world (variant, then the provider). */
export function OvioToaster({
  variant,
  position = "bottom-right",
  ...props
}: OvioToasterProps) {
  const world = useWorld(variant);
  const body = useSyncExternalStore(
    noop,
    () => document.body,
    () => null,
  );
  // On body, so a transformed ancestor (a page transition, an animating card) can't become the
  // containing block of Sonner's fixed layer and pull the toasts inside it.
  return body
    ? createPortal(
        <OvioProvider world={world}>
          <Toaster position={position} {...props} />
        </OvioProvider>,
        body,
      )
    : null;
}

const noop = () => () => {};
