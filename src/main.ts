import { mount } from "svelte";
import App from "./App.svelte";
import "./styles.css";

const target = document.getElementById("app");
if (!target) {
  throw new Error("Root #app element not found in DOM");
}

document.body.classList.add("bg-gray-900");
target.classList.add("bg-gray-900");

const app = mount(App, { target });

export default app;

