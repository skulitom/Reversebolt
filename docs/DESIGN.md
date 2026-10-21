# First desktop workbench

User preference, 2026-10-20: dark mode informed by Anode and Agentbolt.
Use the Bolt family's charcoal surfaces, warm amber selection and key actions,
soft borders, rounded controls, and a dark native title bar. Text is off-white
with readable gray secondary labels; green marks verified facts.
Typography is a clear sans serif with generous spacing; numeric observations
use tabular/monospaced values. No decorative chart data or nonfunctional controls.

Reference tokens: Agentbolt's `crates/agentbolt-desk/src/theme.rs` and Anode's
`src/Anode/Daemon/ViewerTheme.cs`. Reversebolt's shared tokens live in
`crates/reversebolt-app/src/theme.rs`.

The main application workspace is a collection of useful controls, not a report.
Selecting an application starts discovery automatically. Each finding appears
as a card with a plain name, a short description, a value or state, and an action
appropriate to what the engine can actually do. Numbers can be dragged or typed
when a trial is available; actions use buttons. Future verified switches and
sliders must use the same shared engine rather than introducing display-only
controls. Do not invent gameplay meanings or editable ranges to fill the layout.

Keep uncertainty and age compact on the card. Put explanations, screenshots,
candidate lists and technical evidence behind Inspect or Investigation details.
Pause and stop remain easy to reach. The left rail holds application selection,
agent connection and optional practice. Practice retains its explicit staged
observe/discover/test/edit flow; it is separate from automatic application use.

The first build's practice environment must be unmissably labeled. It supplies
numeric observations and an arena boundary; it is not screen recognition or an
independent game. Its memory layout and numeric representation are randomized,
and its true field addresses never enter the discovery algorithm.

Desktop actions and MCP calls enter one serialized engine. The GUI remains
responsive while the engine runs a command. Agent actions appear in the same
history and state. Errors use plain descriptions and remain visible.

Review at 1280x720 in Anode: all primary actions readable, no cropped controls,
history and evidence legible, disabled states understandable, keyboard/UIA
labels present. Test actual discover/edit/restore through the displayed UI and
through MCP before delivering the visible desktop build.
