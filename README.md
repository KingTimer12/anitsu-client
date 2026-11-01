# Anitsu Client

Aplicativo desktop não oficial para navegar e baixar do acervo da [Anitsu](https://anitsu.moe/), com capa, sinopse, elenco e ficha técnica de cada título vindas do [AniList](https://anilist.co/).

> [!IMPORTANT]
> **Este projeto não tem nenhuma relação com a Anitsu.**
>
> - Usamos o acervo da Anitsu (`anitsu.moe` e `nuvem.anitsu.moe`), mas o Anitsu Client é independente: não é feito, mantido, apoiado nem endossado pela equipe da Anitsu.
> - **Todo o conteúdo que existe na plataforma é crédito da comunidade** que o reúne, organiza e mantém. Nenhum arquivo é hospedado, armazenado ou redistribuído por este projeto.
> - A finalidade do aplicativo é apenas **facilitar** o acesso ao acervo para quem já tem conta: navegar, buscar e baixar com uma interface mais confortável.
> - Os dados de capa, sinopse, elenco e equipe vêm da API pública do AniList, que também não tem relação com este projeto.

## O que faz

- **Login com Discord pelo navegador:** abre o Discord no navegador em que você já está logado; não pede senha no app.
- **Acervo inteiro:** Animes, Mangás, Doramas, Desenhos e OSTs, com busca rápida (`Ctrl K`).
- **Página de cada obra:** capa, sinopse, gêneros, nota, personagens e dubladores, equipe, obras relacionadas e ficha técnica, em abas.
- **Temporada atual na tela inicial:** os animes da temporada no AniList, com um selo nos que já estão no acervo.
- **Downloads:** episódio por episódio ou "Baixar todos", com fila (2 ao mesmo tempo) e progresso na barra de título.

## Requisitos

- Uma conta na Anitsu (o login é só via Discord e o cadastro de contas novas é feito pela própria Anitsu).
- Para desenvolver: [Bun](https://bun.sh/), [Rust](https://www.rust-lang.org/tools/install) e os [pré-requisitos do Tauri](https://tauri.app/start/prerequisites/) para o seu sistema.

## Como rodar

```sh
bun install
bun tauri dev      # abre o app em modo de desenvolvimento
bun tauri build    # gera o instalador
```

Testes do lado Rust:

```sh
cd src-tauri
cargo test
```

## Como funciona

| Parte | Onde | O que faz |
| --- | --- | --- |
| Autenticação | `src-tauri/src/auth/` | Login OAuth (PKCE) com o Discord via navegador do sistema, renovação e logout da sessão. |
| HTTP | `src-tauri/src/http/` | Cliente com URL base e autenticação embutidas para os serviços da Anitsu. |
| Acervo | `src-tauri/src/nuvem/` | Listagem, busca e download em stream direto para o disco. |
| AniList | `src-tauri/src/anilist/` | Consultas em lote, casamento de nomes de pasta com títulos e cache em disco. |
| Interface | `src/` | SolidJS + Tailwind CSS, com barra de título própria. |

**Privacidade:** os tokens de sessão ficam só no lado Rust e são gravados na pasta de dados do aplicativo no seu computador. Eles nunca são expostos ao JavaScript da interface nem enviados a outro lugar além dos próprios servidores da Anitsu.

**Uso leve das APIs:** consultas ao AniList são agrupadas e guardadas em cache; a checagem de quais animes estão no acervo só acontece quando o card aparece na tela, com poucos pedidos em paralelo.

## Licença

Distribuído sob a [licença MIT](LICENSE). A licença cobre apenas o código deste repositório, não o conteúdo do acervo nem os dados do AniList, que pertencem a quem os criou.
