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

The lock-screen concepts are not implemented yet: the passcode keypad is drawn by the
host (themes must never handle the passcode), and theme styling for it is planned.
