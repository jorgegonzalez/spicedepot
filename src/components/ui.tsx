import { ButtonHTMLAttributes, forwardRef, InputHTMLAttributes } from "react";

type Variant = "primary" | "secondary" | "danger" | "ghost";

const variantClasses: Record<Variant, string> = {
  primary:
    "bg-orange-500 text-white hover:bg-orange-600 disabled:bg-orange-500/40",
  secondary:
    "bg-slate-200 dark:bg-slate-800 text-slate-900 dark:text-slate-100 hover:bg-slate-300 dark:hover:bg-slate-700 border border-slate-300 dark:border-slate-700 disabled:opacity-50",
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
        className={`inline-flex items-center justify-center rounded px-3 py-1.5 text-sm font-medium transition disabled:cursor-not-allowed ${variantClasses[variant]} ${className}`}
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
        className={`w-full rounded border border-slate-300 dark:border-slate-700 bg-slate-100 dark:bg-slate-900 px-2 py-1.5 text-sm placeholder:text-slate-400 dark:placeholder:text-slate-500 focus:border-orange-500 focus:outline-none focus:ring-1 focus:ring-orange-500 ${className}`}
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
      <div className="mb-1 text-xs uppercase tracking-wide text-slate-600 dark:text-slate-400">
        {label}
      </div>
      {children}
      {hint && <div className="mt-1 text-xs text-slate-500">{hint}</div>}
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
    error: "border-red-700/60 bg-red-950/40 text-red-200",
    info: "border-slate-300 dark:border-slate-700 bg-slate-100 dark:bg-slate-900 text-slate-700 dark:text-slate-300",
    success: "border-emerald-700/60 bg-emerald-950/40 text-emerald-200",
  };
  return (
    <div
      className={`rounded border px-3 py-2 text-sm ${tones[tone]}`}
      role="status"
    >
      {children}
    </div>
  );
}
