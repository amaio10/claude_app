# Potential Upgrades — vers un GUI "institutional grade"

Audit basé sur l'état du code à date. Classé par impact.

## P0 — Fondations manquantes (bloquant)
1. **Persistance serveur** (voir `global_chats.md` — SQLite sur Pi 5 + Tailscale).
2. **Virtualisation des messages** — `Conversation.svelte:47` rend tous les messages. Lag dès ~500 msg. Utiliser `svelte-virtual` ou équivalent.
3. **Recherche globale (⌘K)** — aucune recherche dans les convos, pas de command palette.
4. **Edit / delete / regenerate message** — impossible actuellement. Standard ChatGPT/Claude.
5. **Export conversation** (markdown / JSON) — `/api/fs/download` existe côté backend, pas d'UI.

## P1 — Polish UX
6. **Toasts** — erreurs affichées inline seulement. Ajouter store `toasts.svelte.ts` + `<Toaster />`.
7. **Copy-to-clipboard** sur code blocks et messages entiers.
8. **Raccourcis clavier globaux** :
   - ⌘N  new chat
   - ⌘K  search
   - ⌘/  command palette
   - ⌘,  settings
   - ?    shortcuts modal
9. **Reconnect WebSocket avec backoff** — `ws.ts:44` a un timeout fixe 1500ms, pas de message queue → sends perdus si offline.
10. **Panel Settings** — pas de toggle thème manuel, pas de prefs persistées (font size, sidebar width).
11. **Retry / offline queue** — si le backend tombe pendant un prompt, le message est perdu.

## P2 — Accessibilité
12. **Focus management** — modals (SlashMenu, PathPicker, NotesPanel) sans focus trap, sans Esc global, sans retour focus au trigger.
13. **ARIA** — seulement 4 `aria-label` dans le code. Manque :
    - `aria-live` pour le streaming
    - `aria-expanded` / `aria-haspopup` sur les toggles
    - `role="dialog"` sur les modals
14. **Focus visible** — pas d'outline custom, dépend du browser.
15. **Skip-to-content link** + navigation clavier sidebar (↑↓ entre chats).

## P3 — Responsive / Mobile
16. **Zéro breakpoint responsive** (1 seul match `md:|lg:|sm:`). Sidebar collapse mais layout global casse < 768px.
17. **Bottom sheet mobile** pour les panels (notes, terminal).

## P4 — Features qui élèvent le niveau
18. **Multi-tab / split view** — plusieurs convos ouvertes en parallèle.
19. **Drag-to-reorder** des chats + dossiers / tags.
20. **Usage dashboard** — tokens Groq consommés, coût estimé.
21. **Auth + multi-user** (quand exposé publiquement).
22. **Fichiers pinnés** à une conversation.
23. **Markdown editor WYSIWYG** — Milkdown/Crepe déjà en deps, pas utilisé partout.

## P5 — Perf / Tech
24. **Bundle splitting** — Three.js, KaTeX, Shiki, xterm, Crepe tous dans le graph. Vérifier avec `vite build --report`. Three.js devrait être lazy sur STL uniquement.
25. **Rate limiting backend** pour protéger la clé Groq.
26. **Streaming markdown incrémental** — `MarkdownView.svelte:20` re-parse tout à chaque delta.

---

## Roadmap suggérée
Saut qualitatif visible rapide (~2-3 jours) : **P0 (persistance) + P1 (toasts, copy, ⌘K/⌘N, settings)**.
Ensuite P2 (accessibilité) → P3 (responsive) → P4/P5 (long terme).
