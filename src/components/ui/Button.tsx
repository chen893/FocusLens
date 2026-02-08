import React from 'react';

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'outline' | 'ghost' | 'danger';
  icon?: React.ReactNode;
  loading?: boolean;
}

export const Button: React.FC<ButtonProps> = ({
  children,
  variant = 'outline',
  icon,
  loading,
  className = '',
  ...props
}) => {
  const isIconOnly = React.Children.count(children) === 0;

  const getVariantClass = () => {
    switch (variant) {
      case 'primary': return 'btn-primary';
      case 'danger': return 'danger';
      case 'outline': return 'btn-outline';
      case 'ghost': return 'btn-ghost';
      default: return 'btn-outline';
    }
  };

  return (
    <button
      className={[
        "app-button",
        getVariantClass(),
        isIconOnly ? "app-button-icon-only" : "",
        loading ? "app-button-loading" : "",
        className
      ].filter(Boolean).join(" ")}
      aria-busy={loading || undefined}
      type={props.type ?? "button"}
      disabled={loading || props.disabled}
      {...props}
    >
      {loading ? (
        <span className="button-spinner" aria-hidden="true">◌</span>
      ) : icon}
      {children}
    </button>
  );
};
