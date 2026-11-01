# Higanbana Client

Aplicativo desktop não oficial para navegar e baixar do acervo da [Anitsu](https://anitsu.moe/), com capa, sinopse, elenco e ficha técnica de cada título vindas do [AniList](https://anilist.co/).

> [!IMPORTANT]
> **Este projeto não tem nenhuma relação com a Anitsu.**
>
> - Usamos o acervo da Anitsu (`anitsu.moe` e `nuvem.anitsu.moe`), mas o Higanbana Client é independente: não é feito, mantido, apoiado nem endossado pela equipe da Anitsu.
> - **Todo o conteúdo que existe na plataforma é crédito da comunidade** que o reúne, organiza e mantém. Nenhum arquivo é hospedado, armazenado ou redistribuído por este projeto.
> - A finalidade do aplicativo é apenas **facilitar** o acesso ao acervo para quem já tem conta: navegar, buscar e baixar com uma interface mais confortável.
> - Os dados de capa, sinopse, elenco e equipe vêm da API pública do AniList, que também não tem relação com este projeto.

## O que faz

- **Login com Discord pelo navegador:** abre o Discord no navegador em que você já está logado; não pede senha no app.
- **Acervo inteiro:** Animes, Mangás, Doramas, Desenhos e OSTs, com busca rápida (`Ctrl K`).
- **Página de cada obra:** capa, sinopse, gêneros, nota, personagens e dubladores, equipe, obras relacionadas e ficha técnica, em abas.
- **Temporada atual na tela inicial:** os animes da temporada no AniList, com um selo nos que já estão no acervo.
- **Assistir sem baixar:** o player toca direto da nuvem, buscando só os pedaços perto de onde você está (nada vai para o disco). Toca MKV, HEVC, várias faixas de áudio e legendas `.ass` estilizadas, com:
  - botão de pular abertura;
  - contagem para o próximo episódio, que pode ser desligada;
  - continuar de onde parou;
  - legenda e áudio lembrados por release.
- **Downloads:** episódio por episódio ou "Baixar todos", com fila (2 ao mesmo tempo) e progresso na barra de título.

## Requisitos

- Uma conta na Anitsu (o login é só via Discord e o cadastro de contas novas é feito pela própria Anitsu).
- Para desenvolver: [Bun](https://bun.sh/), [Rust](https://www.rust-lang.org/tools/install) e os [pré-requisitos do Tauri](https://tauri.app/start/prerequisites/) para o seu sistema.

## Como rodar

### Chave do Supabase

O app precisa da chave `anon` pública que o próprio site da Anitsu usa no login. Por respeito aos desenvolvedores da Anitsu, ela **não fica neste repositório**: cada pessoa configura a sua localmente.

1. Copie `.env.example` para `.env` na raiz do projeto (o `.env` está no `.gitignore`).
2. Preencha `ANITSU_SUPABASE_ANON_KEY` com a chave. Também pode ser uma variável de ambiente com o mesmo nome.

No build, a chave é embutida no executável mascarada, sem aparecer em texto puro. Sem ela, o build para com uma mensagem explicando o que falta.

### Motor de vídeo (mpv)

O player usa o [mpv](https://mpv.io/) (libmpv) por meio do [tauri-plugin-libmpv](https://github.com/nini22P/tauri-plugin-libmpv). As DLLs não ficam no repositório. Para baixá-las para `src-tauri/lib/` (Windows), rode:

```sh
cd src-tauri
bunx tauri-plugin-libmpv-api setup-lib
```

O script baixa o `libmpv-wrapper` e uma build **LGPL** da libmpv ([zhongfly/mpv-winbuild](https://github.com/zhongfly/mpv-winbuild)). Se as DLLs ficarem em `src-tauri/src-tauri/lib/`, mova-as para `src-tauri/lib/`.

### Comandos

```sh
bun install
bun tauri dev      # abre o app em modo de desenvolvimento
bun tauri build    # gera o instalador
```

Testes:

```sh
bun run test            # lógica do player (vitest)
cd src-tauri && cargo test
```

## Como funciona

| Parte | Onde | O que faz |
| --- | --- | --- |
| Autenticação | `src-tauri/src/auth/` | Login OAuth (PKCE) com o Discord via navegador do sistema, renovação e logout da sessão. |
| HTTP | `src-tauri/src/http/` | Cliente com URL base e autenticação embutidas para os serviços da Anitsu. |
| Acervo | `src-tauri/src/nuvem/` | Listagem, busca e download em stream direto para o disco. |
| AniList | `src-tauri/src/anilist/` | Consultas em lote, casamento de nomes de pasta com títulos e cache em disco. |
| Player | `src-tauri/src/player/` | Proxy local de stream (Range → nuvem, sessão só no Rust), AniSkip com cache e progresso/configurações em `player.json`. |
| Interface | `src/` | SolidJS + Tailwind CSS, com barra de título própria. |

**Privacidade:** os tokens de sessão ficam só no lado Rust e são gravados na pasta de dados do aplicativo no seu computador. Eles nunca são expostos ao JavaScript da interface nem enviados a outro lugar além dos próprios servidores da Anitsu.

**Uso leve das APIs:** consultas ao AniList são agrupadas e guardadas em cache; a checagem de quais animes estão no acervo só acontece quando o card aparece na tela, com poucos pedidos em paralelo.

## Créditos

- [mpv](https://mpv.io/): motor de vídeo, usado como biblioteca dinâmica (build LGPL; licença do mpv: LGPL-2.1+ / GPL-2.0+ conforme a build).
- [tauri-plugin-libmpv](https://github.com/nini22P/tauri-plugin-libmpv) (MPL-2.0).
- [AniSkip](https://aniskip.com/): tempos de abertura e encerramento enviados pela comunidade, usados quando o arquivo não tem capítulos.

## Licença

Distribuído sob a [licença MIT](LICENSE). A licença cobre apenas o código deste repositório, não o conteúdo do acervo nem os dados do AniList, que pertencem a quem os criou.
