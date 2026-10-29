import { mount } from "svelte";
import App from "./App.svelte";
import LiveMayhemOverlay from "./lib/components/overlay/LiveMayhemOverlay.svelte";
import "./styles.css";

const target = document.getElementById("app");
if (!target) {
  throw new Error("Root #app element not found in DOM");
}

const isOverlay = new URLSearchParams(window.location.search).get("window") === "overlay";

if (isOverlay) {
  document.documentElement.classList.add("overlay-window");
  document.body.classList.add("overlay-window");
  target.classList.add("overlay-window");

  document.documentElement.style.background = "transparent";
  document.documentElement.style.backgroundColor = "rgba(0, 0, 0, 0)";
  document.body.style.background = "transparent";
  document.body.style.backgroundColor = "rgba(0, 0, 0, 0)";
  target.style.background = "transparent";
  target.style.backgroundColor = "rgba(0, 0, 0, 0)";
} else {
  document.body.classList.add("bg-gray-900");
  target.classList.add("bg-gray-900");
}

const app = mount(isOverlay ? LiveMayhemOverlay : App, {
  target,
});

export default app;

