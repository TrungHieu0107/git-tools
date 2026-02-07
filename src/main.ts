import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";

try {
  const app = mount(App, {
    target: document.getElementById("app")!,
  });
  // @ts-ignore
  window.app = app;
  
  // Remove loading overlay
  const loading = document.getElementById("loading");
  if (loading) loading.remove();
} catch (err) {
  document.body.innerHTML = `<div style="color: red; padding: 20px;">
    <h1>App Trace Error</h1>
    <pre>${err}</pre>
    <pre>${(err as Error).stack}</pre>
  </div>`;
  console.error("Failed to mount app:", err);
}

export default {};
