// Pulls the Tailwind-compiled stylesheet into the bundle. We don't ship any
// runtime JS yet — the FAQ uses native <details>/<summary>, the smooth
// scroll uses `scroll-behavior: smooth`, and the mobile menu fits inside
// the header (no toggle needed for the current nav size). If we add an
// interactive bit later, this is where it lands.
import "./style.css";
