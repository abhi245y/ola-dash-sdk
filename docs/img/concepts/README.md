# Concept art

AI-generated concept images (made with an image model from written prompts) that set
the direction for the three reference themes. They are mood and layout references,
not screenshots: the shipped themes are in `themes/` and their real renders are in
`docs/img/themes/`.

| Theme | Dash concept | Lock-screen concept |
|---|---|---|
| Segment (`themes/segment`) | `segment-dash.jpg` | `segment-lockscreen.jpg` |
| Data Fields (`themes/fields`) | `fields-dash.jpg` | `fields-lockscreen.jpg` |
| Default (`themes/default`) | `default-dash.jpg` | `default-lockscreen.jpg` |

The lock-screen concepts are implemented through the optional `lock-style` property:
the theme supplies colours, key shape and digit style, while the host draws the keypad
and handles the passcode (see `docs/themes.md` and `docs/safety.md`). Real renders:
`docs/img/themes/*-locked.png`.
