# Transcrição — MiniMax e camada de provedor

## Como está implementado (PR5)
A transcrição é uma **camada plugável** e **configurável pela UI** (Configurações):
- **Endpoint de transcrição** (URL completa) — campo editável.
- **Modelo** (campo `model`) — campo editável.
- **Chave da API** — guardada no **keychain do SO** (crate `keyring`), nunca em texto puro nem no SQLite.
- **Idioma** — por transcrição, padrão `pt`.

Provedor concreto: `OpenAiCompatible` (`transcription/mod.rs`) — faz `POST {endpoint}` multipart com `file` + `model` + `language` + `response_format=json`, header `Authorization: Bearer <chave>`, e lê `{"text": "..."}` da resposta. Cobre OpenAI/Groq Whisper e qualquer endpoint compatível.

## MiniMax — chave de assinatura (sk-cp)
A chave é a **Subscription Key da MiniMax (prefixo `sk-cp`)**, do plano de tokens (token-plan), **não** uma chave pay-as-you-go. Ela é enviada como **Bearer token** no header `Authorization` — exatamente o que o provedor já faz. Então, do ponto de vista do código, a chave `sk-cp` é tratada como qualquer Bearer token: o usuário cola em Configurações → vai pro keychain → enviada nas requisições.

## A confirmar (bloqueia o teste real com MiniMax)
Não consegui ler as docs da MiniMax neste ambiente (WebFetch/WebSearch quebrados aqui). Falta confirmar e preencher em Configurações:
1. **URL do endpoint de speech-to-text da MiniMax** (a doc enviada é `token-plan/quickstart`, sobre a assinatura, não o ASR).
2. **Nome do modelo** de transcrição.
3. **Formato do request**: a MiniMax aceita multipart `file` igual à OpenAI? Ou usa upload de arquivo → `file_id` → job assíncrono com polling? Se for assíncrono, precisa de um provedor novo (não cabe no `OpenAiCompatible`).
4. **GroupId**: a `sk-cp` é escopada por conta/assinatura, então talvez dispense `GroupId`. Confirmar. Se precisar, dá pra embutir como query na URL do endpoint.
5. **Confirmar que a MiniMax tem ASR.** Se não tiver, fallback: OpenAI ou Groq Whisper (basta apontar o endpoint/modelo em Configurações).

## Como configurar/testar
Configurações → preencher Endpoint, Modelo e Chave → Salvar. Aba Transcrição → escolher gravação + idioma → Transcrever.
- Default de fábrica aponta pra OpenAI (`/v1/audio/transcriptions`, `whisper-1`) só pra ter um caminho que funciona; troque pelos valores da MiniMax quando confirmados.

## Segurança
- Chave no keychain; nunca logar áudio nem chave.
- Avisar o usuário (UI) que a transcrição envia o áudio para o provedor configurado.
