/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./src/**/*.{html,js,svelte,ts}"
  ],
  /* https://www.ockam.io/style-guide */
  theme: {
    colors: {
      natural: {
        dark: "#242A31",
        neutral: "#7A8895",
        medium: "#D1DBE3",
        light: "#EFF1F1",
        background: "#F9F9F9",
        white: "#FFFFFF"
      },
      primary: {
        deep: "#0A1A2B",
        accent: "#162535",
        ockam: "#52C7EA",
        "dark-ockam": "#36A7C9"
      },
      "primary-gradient": {
        start: "#4FDAB8",
        end: "#52C7EA"
      },
      secondary: {
        light: "#4FDAB8",
        avocado: "#3AC6A3",
        dark: "#1D5B58",
        pastel: "#A0F6E1",
        azure: "#ECFDF9"
      },
      "secondary-gradient": {
        start: "#36A7C9",
        end: "#3AC6A3"
      },
      tertiary: {
        orange: "#EC432D"
      }
    },
    extend: {},
    fontFamily: {
      brand: ["Inter", "ui-serif", "Georgia", "Cambria", "Times\\ New\\ Roman", "Times", "serif"]
    }
  },
  plugins: [],
}

