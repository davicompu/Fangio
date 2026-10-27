import React from "react";
import ReactDOM from "react-dom/client";
import "./styles.css";
import App from "./App";
import { IS_MAC } from "./lib/utils";

// Platform class: CSS uses it to reserve space for the Mac overlay title bar.
document.documentElement.classList.add(IS_MAC ? "platform-mac" : "platform-other");

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
