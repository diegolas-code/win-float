# Developer Agent Guidelines

Welcome, developer agent. Please follow these guidelines strictly when working on the `win-float` project.

## 0. General Principles
- You're an expert Rust developer with deep knowledge of Windows API and experience in building system utilities.

## 1. Safety & Boundaries
- Always respect the directives defined in the global `GEMINI.md` file concerning git operations, workspace boundaries, security, and dependencies.
- Do not perform any destructive git commands unprompted.

## 2. Test-Driven Development (TDD)
- **Iron Law:** Write the test first, verify that it fails, write minimal code to pass, verify green, and then refactor.
- If you write production code before a test exists, delete it and start over.
- Keep tests focused on testing real behavior, not mocks (unless mocking Win32 OS calls).

## 3. Architecture & Decoupling
- Win32 API interactions are complex and hard to test. Keep all Win32 API calls isolated behind the traits in [traits.rs](file:///C:/Users/Diegolas/Code/rust/WIN-FLOAT/win-float/src/traits.rs).
- Business logic (state machine, coordinate maths, step size math) must never call Win32 functions directly. It must only interact through mockable traits.

## 4. Workflow Checkpoints
- Prior to starting any task, read the current status in [TODO.md](file:///C:/Users/Diegolas/Code/rust/WIN-FLOAT/win-float/TODO.md) and the last handoff state in [PAUSE.md](file:///C:/Users/Diegolas/Code/rust/WIN-FLOAT/win-float/PAUSE.md).
- Update [TODO.md](file:///C:/Users/Diegolas/Code/rust/WIN-FLOAT/win-float/TODO.md) to check off tasks as soon as they are completed.
- Commit atomically after passing each test step.
- For each major commit, create a new tracking file in [.history/](file:///C:/Users/Diegolas/Code/rust/WIN-FLOAT/win-float/.history/) (following the template in [.history/README.md](file:///C:/Users/Diegolas/Code/rust/WIN-FLOAT/win-float/.history/README.md)) documenting tasks completed, issues found, solutions, and verification proof.
- Update [PAUSE.md](file:///C:/Users/Diegolas/Code/rust/WIN-FLOAT/win-float/PAUSE.md) at the end of every session to ensure a clean handoff of project state.