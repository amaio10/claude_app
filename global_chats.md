# Persistance globale des conversations

## Problème
- Backend : sessions en RAM uniquement (`DashMap` dans `backend/src/state.rs:12`). Rien n'est persisté côté serveur.
- Frontend : tout est dans `localStorage` (`claude_app.chats.v1`, `frontend/src/lib/stores.svelte.ts:20-47`).
- Conséquence : conversations liées au navigateur. Perdues si on vide le cache, change de browser, ou de machine.

## Objectif
Retrouver toutes les conversations depuis n'importe quel device (Mac, téléphone, autre PC).

## Solution retenue : Raspberry Pi 5 + SQLite + Tailscale

### Pourquoi
- Pi 5 largement suffisant pour ce workload (proxy Groq + SQLite).
- Tailscale = accès depuis partout sans exposer de port sur internet, pas besoin d'auth/TLS pour une app perso.
- Clé Groq reste côté Pi, jamais dans le frontend.

### Plan backend
1. Ajouter `rusqlite` (ou `sqlx`) dans `backend/Cargo.toml`.
2. Schéma :
   - `conversations(id, label, cwd, claude_session_id, created_at, updated_at)`
   - `messages(id, conversation_id, role, text, ts)`
3. Endpoints à ajouter :
   - `GET  /api/conversations`
   - `GET  /api/conversations/:id`
   - `POST /api/conversations` (create)
   - `POST /api/conversations/:id/messages` (append)
   - `DELETE /api/conversations/:id`
4. Binaire écoute sur `0.0.0.0:7777`.

### Plan frontend
- Remplacer (ou doubler) le `localStorage` par des appels aux endpoints ci-dessus.
- Au boot : fetch `/api/conversations` au lieu de lire `localStorage`.

### Déploiement Pi
- Rust cross-compile ou build direct sur le Pi (ARM64).
- `systemd` service pour le backend.
- `.env` avec `GROQ_API_KEY` reste sur le Pi.
- Tailscale installé sur Pi + Mac + téléphone → URL type `http://raspberrypi:7777`.

### Deux options d'archi
- **(a) Frontend sur Mac, backend+DB sur Pi** : simple, le Mac fait `pnpm run dev` et pointe vers le Pi. Inconvénient : besoin du Mac pour accéder.
- **(b) Tout sur le Pi** : `vite build` statique servi par le backend (ou un reverse proxy léger type Caddy). Une seule URL accessible partout via Tailscale. Plus élégant, un peu plus de setup.

Recommandation : **(b)**.

## Tradeoffs / points d'attention
- Pi doit rester allumé (~5W, négligeable).
- Si DB sur SD card : prévoir backup régulier du `.sqlite` (les SD meurent). Sur SSD USB : rien à faire.
- Pas d'auth prévue : on s'appuie sur Tailscale pour l'isolation réseau. Si un jour on veut exposer publiquement → ajouter auth + TLS (Caddy gère les certs auto).
