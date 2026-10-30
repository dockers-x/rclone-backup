import React from "react";
import { setNonce } from "get-nonce";
import "sonner/dist/styles.css";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./styles/app.css";
class ErrorBoundary extends React.Component {
  state = { error: null };
  static getDerivedStateFromError(error) {
    return { error };
  }
  render() {
    if (this.state.error)
      return (
        <div role="alert" className="fatal-error">
          <h1>Rclone Backup</h1>
          <p>页面加载失败 / Unable to load the workspace</p>
          <button onClick={() => location.reload()}>重新加载 / Reload</button>
        </div>
      );
    return this.props.children;
  }
}
document.addEventListener(
  "keydown",
  () => {
    document.documentElement.dataset.input = "keyboard";
  },
  true,
);
document.addEventListener(
  "pointerdown",
  () => {
    document.documentElement.dataset.input = "pointer";
  },
  true,
);
const theme = localStorage.getItem("theme");
if (["system", "light", "dark"].includes(theme))
  document.documentElement.dataset.theme = theme;
const nonce = document.querySelector('meta[name="csp-nonce"]')?.content;
if (nonce && !nonce.startsWith("{{")) setNonce(nonce);
createRoot(document.getElementById("root")).render(
  <ErrorBoundary>
    <App />
  </ErrorBoundary>,
);
