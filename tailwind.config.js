/** @type {import('tailwindcss').Config} */
module.exports = {
  darkMode: ["class"],
  content: {
    files: ["./src/**/*.{rs,html}", "./.dioxus/**/*.{html,js}", "./index.html"],
    extract: {
      rs: (content) => {
        const matches = Array.from(content.matchAll(/class:\s*"([^"]+)"/g)).map(([, group]) => group);
        const class_attr = Array.from(content.matchAll(/class="([^"]+)"/g)).map(([, group]) => group);
        return matches.concat(class_attr);
      },
    },
  },
  theme: {
    extend: {},
  },
  plugins: [require("tailwindcss-animate")],
};
