import App from "@/App.tsx";
import "@/index.css";
import { Decimal } from "decimal.js";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

// Configure Decimal.js for ultra-high precision calculations throughout the app
// This supports zoom levels up to 10^100+ without precision loss
Decimal.set({ precision: 300 });

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>
);
