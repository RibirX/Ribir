# Ribir Gallery

Welcome to the Ribir Gallery! This application serves as the official interactive showroom and learning resource for the [Ribir](https://github.com/RibirX/Ribir) declarative GUI framework. 

The gallery is designed with a specific goal in mind: to attract developers by demonstrating Ribir's capabilities, its elegant declarative syntax, and its powerful underlying architecture, all through a modern, expressive UI.

## Architecture & Organization

To provide a clear and engaging learning path for developers — **"See the results -> Find the tools -> Experience the expressiveness -> Understand the core"** — the Gallery is structured into four primary top-level sections:

### 1. Showcase
**Goal:** Make a strong first impression by demonstrating complex, real-world applications built with Ribir. This breaks the "toy framework" stereotype.
- **Content:** Complete, interactive applications (e.g., Wordle Game, Messages App, Pomodoro Timer). These examples highlight state management (`$state`), complex layouts, and performance in a real-world context.
- **Presentation (Showroom Mode):** A split-view approach. Users interact with the live application on one side, while the other side highlights the core declarative code (`fn_widget!`) or state management logic that powers it. We show the *essence*, not the entire source file.

### 2. Widgets
**Goal:** Serve as a practical, interactive dictionary of Ribir's built-in components, proving the framework's readiness for production.
- **Content:** Categorized standard UI building blocks (Inputs, Data Display, Navigation, Layouts, etc.), primarily based on Material Design principles.
- **Presentation:** Interactive previews showing components in various states (Normal, Hover, Pressed, Disabled) alongside clean, copy-pasteable declarative code snippets.

### 3. Animations
**Goal:** Highlight Ribir's unique, zero-boilerplate approach to smooth, performant animations. Fluid UI is a key metric for modern frameworks.
- **Content:** Examples of transitions, state-driven motion, easing curves, and complex choreographed animations.
- **Presentation:** Demonstrates how minimal declarative code (often just a few lines of `Transition` configuration) is required to achieve high-fps, sophisticated visual feedback, without getting bogged down in lifecycle management.

### 4. Concepts
**Goal:** Explain the "Why" behind Ribir to hardcore Rust developers, focusing on its elegant engineering and resolving common skepticism.
- **Content:** Deep dives into Ribir's core pillars:
  - **Reactivity:** How precise, granular updates are achieved without a Virtual DOM.
  - **Fat Objects:** Why you don't need endless wrapper `Container`s to apply styles like `margin`, `radius`, or `background`.
  - **Declarative Macros:** How `fn_widget!` flattens UI trees with zero-cost abstraction.
- **Presentation:** Interactive diagrams or minimal, focused runnable demos paired with concise, hard-hitting text explanations.

## Adding New Content

When adding new examples, components, or features to the Gallery, please ensure they fit into one of these four pillars and adhere to the following guidelines:

1. **Focus on the Code Essence:** The Gallery is for developers. Always pair the visual UI with the most relevant, elegant piece of Ribir code that makes it work. Avoid dumping unformatted source files.
2. **Maintain the Design Language:** The Gallery itself uses a Material 3 Expressive "Floating Shell" layout. Ensure new pages adhere to this aesthetic (large rounded corners, distinct background container roles, high signal-to-noise ratio).
3. **Keep it Interactive:** Whenever possible, let the user click, type, or hover to see how Ribir responds. Static images should be a last resort.
