/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      fontFamily: {
        sans: ['"IBM Plex Sans"', "Segoe UI", "sans-serif"],
        mono: ['"IBM Plex Mono"', "ui-monospace", "monospace"],
      },
      colors: {
        ink: {
          950: "#0b1220",
          900: "#121a2b",
          700: "#2a3a55",
          500: "#6b7c99",
          100: "#e8eef8",
        },
        accent: {
          DEFAULT: "#1f6feb",
          soft: "#3d8bfd",
        },
      },
    },
  },
  plugins: [],
};
