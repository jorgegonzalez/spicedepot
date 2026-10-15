import { ButtonHTMLAttributes, forwardRef, InputHTMLAttributes, useEffect, useId, useRef } from "react";
import { createPortal } from "react-dom";

type Variant = "primary" | "secondary" | "danger" | "ghost";

const variantClasses: Record<Variant, string> = {
  primary:
    "border border-orange-400/70 bg-orange-500 text-white shadow-[0_8px_28px_rgba(249,115,22,0.22)] hover:bg-orange-400 hover:shadow-[0_10px_34px_rgba(249,115,22,0.3)] disabled:bg-orange-500/40",
  secondary:
    "bg-slate-100 dark:bg-slate-900/80 text-slate-900 dark:text-slate-100 hover:bg-slate-200 dark:hover:bg-slate-800 border border-slate-300 dark:border-slate-700 disabled:opacity-50",
  danger:
    "bg-red-600/90 text-white hover:bg-red-600 disabled:bg-red-600/40",
  ghost: "text-slate-700 dark:text-slate-300 hover:bg-slate-200/60 dark:hover:bg-slate-800/60 disabled:opacity-50",
};

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  function Button({ variant = "primary", className = "", ...rest }, ref) {
    return (
      <button
        ref={ref}
        className={`inline-flex min-h-9 items-center justify-center rounded-[10px] px-4 py-2 text-sm font-semibold transition duration-200 disabled:cursor-not-allowed ${variantClasses[variant]} ${className}`}
        {...rest}
      />
    );
  },
);

export const Input = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(
  function Input({ className = "", ...rest }, ref) {
    return (
      <input
        ref={ref}
        className={`w-full rounded-[11px] border border-slate-300 dark:border-slate-700 bg-white/80 dark:bg-slate-950/70 px-3 py-2.5 text-sm shadow-inner shadow-black/5 placeholder:text-slate-400 dark:placeholder:text-slate-500 focus:border-orange-500 focus:outline-none focus:ring-2 focus:ring-orange-500/20 ${className}`}
        {...rest}
      />
    );
  },
);

export function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <label className="block">
      <div className="mb-1.5 text-[11px] font-semibold uppercase tracking-[0.12em] text-slate-600 dark:text-slate-400">
        {label}
      </div>
      {children}
      {hint && <div className="mt-1.5 text-xs text-slate-500">{hint}</div>}
    </label>
  );
}

export function Banner({
  tone,
  children,
}: {
  tone: "error" | "info" | "success";
  children: React.ReactNode;
}) {
  const tones = {
    error: "border-red-300 bg-red-50 text-red-800 dark:border-red-700/60 dark:bg-red-950/40 dark:text-red-200",
    info: "border-slate-300 dark:border-slate-700 bg-slate-100 dark:bg-slate-900 text-slate-700 dark:text-slate-300",
    success: "border-emerald-300 bg-emerald-50 text-emerald-800 dark:border-emerald-700/60 dark:bg-emerald-950/40 dark:text-emerald-200",
  };
  return (
    <div
      className={`rounded-xl border px-4 py-3 text-sm shadow-sm ${tones[tone]}`}
      role="status"
    >
      {children}
    </div>
  );
}

/** Native modal provides focus trapping, inert background, and Escape behavior. */
export function Dialog({ title, children, onClose, busy = false }: {
  title: string; children: React.ReactNode; onClose: () => void; busy?: boolean;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  const titleId = useId();
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    const dialog = ref.current!;
    dialog.showModal();
    return () => { dialog.close(); previous?.focus(); };
  }, []);
  return createPortal(
    <dialog ref={ref} aria-labelledby={titleId} aria-modal="true"
      onCancel={(event) => { event.preventDefault(); if (!busy) onClose(); }}
      className="w-[calc(100%_-_3rem)] max-w-md rounded-2xl border border-slate-300 bg-white p-6 text-slate-900 shadow-2xl backdrop:bg-slate-950/70 dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100">
      <h2 id={titleId} className="text-lg font-semibold">{title}</h2>
      {children}
    </dialog>, document.body,
  );
}
