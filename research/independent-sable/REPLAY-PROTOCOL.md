# Development replay after request accounting and context sizing

Freeze current Rust sources, the built `reversebolt-runtime-hypotheses`
executable, this protocol and the unchanged native Sable report before inference.
Run the existing CLI once with qwen3.8:latest, no opaque transformation and no
activity image input. No game, target read, screenshot, input or field write.
Preserve every failure and do not retry or adjust prompts based on output.

The production change uses a 16,384-token context only when the complete prompt
byte length plus the unchanged output allowance and 512-token headroom fits.
Larger requests retain their previous context; post-response input accounting
still rejects possible truncation. No evidence, field or proposed question is
removed to fit. Failed stages retain their full prompt/schema/options, elapsed
time, phase, error and any response received before an identity-check failure.
Selection and naming prompts, quotas, citations and edit guards are unchanged.

Report selected types, proposal groups, retained/rejected hypotheses, request
contexts, errors and latency. This is a development replay of a now-known failed
target. Game-closed success does not prove the native timeout is fixed or any
gameplay control works. Keep the original native failure and the four context
diagnostic responses intact. No new capability percentage is earned solely by
producing names from saved data.
