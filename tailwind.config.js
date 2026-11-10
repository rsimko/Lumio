/** @type {import('tailwindcss').Config} */
const scaled = (size, lineHeight) => [
  `calc(${size} * var(--font-scale, 1))`,
  `calc(${lineHeight} * var(--font-scale, 1))`,
];

export default {
  darkMode: 'class',
  content: ['./index.html', './src/**/*.{vue,ts}'],
  theme: {
    extend: {
      colors: {
        app: 'rgb(var(--c-app) / <alpha-value>)',
        panel: 'rgb(var(--c-panel) / <alpha-value>)',
        'panel-2': 'rgb(var(--c-panel-2) / <alpha-value>)',
        elevated: 'rgb(var(--c-elevated) / <alpha-value>)',
        main: 'rgb(var(--c-main) / <alpha-value>)',
        canvas: 'rgb(var(--c-canvas) / <alpha-value>)',
        line: 'rgb(var(--c-line) / <alpha-value>)',
        'line-strong': 'rgb(var(--c-line-strong) / <alpha-value>)',
        ink: 'rgb(var(--c-ink) / <alpha-value>)',
        muted: 'rgb(var(--c-muted) / <alpha-value>)',
        subtle: 'rgb(var(--c-subtle) / <alpha-value>)',
        accent: 'rgb(var(--c-accent) / <alpha-value>)',
        'accent-soft': 'rgb(var(--c-accent-soft) / <alpha-value>)',
        'accent-ink': 'rgb(var(--c-accent-ink) / <alpha-value>)',
        danger: 'rgb(var(--c-danger) / <alpha-value>)',
        warning: 'rgb(var(--c-warning) / <alpha-value>)',
        success: 'rgb(var(--c-success) / <alpha-value>)',
      },
      fontFamily: {
        sans: ['var(--font-ui)'],
        mono: ['var(--font-mono)'],
      },
      fontSize: {
        '3xs': scaled('8px', '10px'),
        '2xs': scaled('10px', '14px'),
        ui: scaled('13px', '18px'),
        xs: scaled('12px', '16px'),
        sm: scaled('14px', '20px'),
        base: scaled('16px', '24px'),
        lg: scaled('18px', '28px'),
        xl: scaled('20px', '28px'),
        '2xl': scaled('24px', '32px'),
        '3xl': scaled('30px', '36px'),
      },
      boxShadow: {
        panel: '0 1px 2px rgb(0 0 0 / 0.06), 0 8px 24px rgb(0 0 0 / 0.08)',
        float: '0 8px 30px rgb(0 0 0 / 0.22)',
      },
      transitionDuration: {
        DEFAULT: '150ms',
      },
    },
  },
  plugins: [],
};
