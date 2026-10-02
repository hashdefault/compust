# Site do Compust

[English (US)](README.md) | [Português (Brasil)](README.pt-BR.md)

O site público do projeto é publicado em <https://hashdefault.github.io/compust/>.
O inglês fica em `/compust/`; o português brasileiro, em `/compust/pt-br/`.
Os dois idiomas incluem primeiros passos, arquitetura, roteiro e guia de
contribuição. Os guias são gerados a partir dos arquivos Markdown do repositório:
edite esses arquivos, em vez do HTML gerado.

## Trabalhe no site

Use Node.js 24 ou mais recente. Na raiz do repositório:

```sh
npm ci --ignore-scripts
npm test
npm run build
npm run preview
```

Abra <http://127.0.0.1:4173/compust/>. A prévia serve `site/dist/`, incluindo o
caminho do projeto usado no GitHub Pages. Compile novamente após cada alteração;
não há recarga automática. `PORT=4174 npm run preview` seleciona outra porta.
Encerre com Ctrl+C.

`content.mjs` contém os textos da página inicial nos dois idiomas. `templates.mjs`
define a navegação compartilhada, a página inicial, os guias e a página 404.
`build.mjs` converte o Markdown, resolve os links relativos dos documentos,
identifica os recursos por seu conteúdo e gera o sitemap. `assets/` contém o
estilo, os controles opcionais da ilustração e o favicon. A única dependência de
compilação é `markdown-it`; o navegador não precisa de dependências nem de
requisições a terceiros.

Siga [DESIGN.md](../DESIGN.md) ao alterar a interface. A página local de componentes
pode ser aberta com `SITE_DIR=site npm run preview`, em
<http://127.0.0.1:4173/compust/primitives.html>. Ela não entra na publicação.

Os testes verificam os links gerados, os caminhos dos idiomas, as âncoras dos
títulos, a existência dos recursos e o tratamento seguro do Markdown. Confira
também os dois idiomas em 375, 768 e 1280px, navegue pelo teclado, experimente os
três controles da ilustração, teste movimento reduzido e confirme que os guias
funcionam com JavaScript desativado.

## Publique

Nas configurações do repositório, em **Settings → Pages → Build and deployment**,
selecione **GitHub Actions** como origem. Essa configuração é feita uma única vez;
o `GITHUB_TOKEN` do workflow não consegue ativar o Pages por conta própria.
O workflow **Website** testa e compila pull requests e publica os pushes para
`main` quando a compilação passa. Também pode ser iniciado manualmente pela aba
Actions. Os arquivos gerados ficam fora do Git.

A URL é de um site de projeto, por isso os links e recursos incluem `/compust/`.
Se o repositório mudar ou receber um domínio próprio, atualize `base` e `origin`
em `templates.mjs` e o caminho da prévia em `serve.mjs`. Verifique as URLs
canônicas, as versões de idioma, o sitemap e os links antes de publicar.

A cena no navegador ilustra os efeitos com CSS. Ela não é um benchmark de
hardware, uma captura real do Compust nem uma indicação de que o compositor
funciona dentro do navegador. Preserve essa distinção nas duas traduções ao
alterar a ilustração ou os textos.
