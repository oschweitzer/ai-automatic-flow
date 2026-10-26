# Button

Buttons trigger one action, and the primary one is lavender.

Use `primary` once per area, for the action that moves the work forward ("Ouvrir le premier", "Valider le plan"). Use `ghost` for links out (Ticket Jira, MR !87, Pipeline) and secondary actions. Pass `href` to render a link. Labels name the action, never "OK" or "Cliquer ici".

The consumer provides the label (in the viewer's language) and the handler or href.
