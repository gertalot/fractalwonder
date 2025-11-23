/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./**/src/**/*.rs", "./index.html"],
  theme: {
    extend: {
      colors: {
        background: "#0a0a0a",
        panel: "#1a1a1a",
        "text-primary": "#e0e0e0",
        "text-secondary": "#a0a0a0",
        accent: "#4a9eff",
      },
    },
  },
  plugins: [],
  // Force Tailwind CSS IntelliSense to use v3 syntax
  future: {
    hoverOnlyWhenSupported: true,
  },
};

