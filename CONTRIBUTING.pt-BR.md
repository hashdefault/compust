# Como contribuir com o Compust

[English (US)](CONTRIBUTING.md) | [Português (Brasil)](CONTRIBUTING.pt-BR.md)

O Compust está no começo: um relato claro de falha, um teste de driver ou uma melhoria pequena e documentada pode orientar o projeto. Issues e pull requests podem ser escritos em português brasileiro ou inglês. Seja específico, respeitoso e explique o que observou.

## Preparar o ambiente

Instale rustup, um linker C, Git e Xvfb. O repositório fixa Rust 1.95.0. Faça um fork, clone seu fork e crie uma branch para uma alteração. Mantenha o compositor da sua área de trabalho em execução enquanto desenvolve em um servidor aninhado.

```sh
cargo build --locked
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace
```

`cargo test` inicia instâncias isoladas do Xvfb com displays escolhidos dinamicamente, executa o binário real e verifica pixels e estado do X11. Não depende do seu `DISPLAY` nem de uma sessão gráfica aberta. Use `XVFB=/caminho/para/Xvfb` para escolher o executável. Para rodar apenas os testes unitários, use `cargo test --bin compust`. Os testes do crate de GPU precisam do EGL do Mesa e são pulados sem o dispositivo de software, a menos que `COMPUST_GPU_TESTS` esteja definida; `COMPUST_GPU_DISPLAY=:0 cargo test --test gpu` também confere o compartilhamento por DRI3 na GPU da sua sessão, usando apenas pixmaps fora da tela.

Os testes aguardam eventos X11 com prazos definidos. Preserve essa abordagem: pausas arbitrárias tornam testes gráficos instáveis. Os testes de animação podem observar o tempo porque a animação é o comportamento verificado. Prefira pixels ou estado do protocolo a verificações do texto de logs ou de detalhes internos.

## Escolher uma contribuição

O [roteiro](docs/ROADMAP.pt-BR.md) separa correção, desempenho e mudanças maiores de renderização. Bons pontos de partida incluem reproduzir uma interação com um gerenciador de janelas, melhorar uma mensagem de diagnóstico, acrescentar um cenário de regressão, testar uma sessão real do XLibre ou aprimorar qualquer idioma da documentação.

Para propor um backend de GPU, um modelo de sincronização ou um formato de configuração novo, abra uma issue explicando as interfaces e os compromissos antes de começar uma implementação grande. Correções pequenas podem ir diretamente para um pull request. Consultar a versão de uma extensão não é suficiente para anunciar suporte ao recurso.

## Relatar um problema

Informe o commit (`git rev-parse HEAD`), distribuição, servidor e versão, gerenciador de janelas, GPU/driver, configuração relevante e a menor reprodução possível. Inclua a saída de `compust --diagnose` e os trechos relevantes dos logs com `RUST_LOG=compust=debug`. Descreva a imagem esperada e a observada; uma captura ou gravação curta ajuda em falhas de renderização.

Em relatos de desempenho, informe resolução, taxas de atualização dos monitores, número de janelas, carga de trabalho, raio do desfoque, configuração e se a medição de CPU inclui o servidor X. O XRender pode transferir trabalho para esse processo. Compare builds de release na mesma máquina e com a mesma carga; ao comparar com picom, registre sua versão e backend.

O [guia de validação de desktops](docs/DESKTOP_TESTING.pt-BR.md) executa cenários do Xmonad, do Openbox ou do i3 nos modos Present, XRender direto e efeitos, em servidores aninhados ou em uma sessão dedicada em hardware, amostra transições físicas de monitores, registra medições dos processos e descreve as verificações de hardware restantes para a beta.

## Preparar um pull request

Mantenha cada pull request concentrado em um comportamento que possa ser explicado e testado. Descreva o que dispara a situação, o resultado, a verificação realizada e as limitações conhecidas. Acrescente um teste de regressão quando ele conseguir distinguir a falha. Rode formatação, Clippy e os testes pertinentes antes do envio; a CI também compila o binário de release.

O crate do compositor proíbe código `unsafe`. O único código unsafe fica em `crates/gl`, atrás de uma API segura, com um comentário `SAFETY` em cada bloco dizendo por que ele é correto; mantenha-o ali e pequeno. Explicite os tempos de vida dos recursos X11, preserve a limpeza quando a inicialização falhar e trate janelas que desaparecem entre requisições. Não amplie a lista de erros de protocolo ignorados apenas para passar um teste. Separe os cálculos de animação da interação com o servidor e reutilize buffers quando possível.

Atualize as versões em inglês e pt-BR quando o comportamento mudar. Se não conseguir traduzir um trecho com segurança, indique no PR qual documento precisa de ajuda. Capturas geradas, `target/`, imagens privadas da área de trabalho e credenciais não devem entrar em commits. Para gerar algumas imagens dos testes:

```sh
COMPUST_ARTIFACTS=artifacts cargo test --test x11
```

O projeto usa a licença [MIT](LICENSE). As contribuições precisam ser compatíveis; identifique código de terceiros e sua licença. Usar o picom como inspiração não autoriza copiar código sob termos incompatíveis.

## Preparar uma versão

As versões continuam como pré-lançamentos até que os critérios do roteiro valham para um escopo maior. Atualize a versão em `Cargo.toml` e `Cargo.lock`, o guia da beta, as notas em `docs/releases/` e os dois idiomas do README e do roteiro. Depois que a CI passar no commit da versão, empacote esse commit e publique com a CLI do GitHub:

```sh
tools/package.sh HEAD artifacts/release
gh release create vX.Y.Z-beta.N artifacts/release/* --prerelease \
    --target "$(git rev-parse HEAD)" --title "Compust X.Y.Z-beta.N" \
    --notes-file docs/releases/vX.Y.Z-beta.N.md
```

Baixe os arquivos publicados e confira `sha256sum -c SHA256SUMS` e `compust --version` antes de anunciar a versão.

## Contribuições para o site

O [site do projeto](https://hashdefault.github.io/compust/pt-br/) publica os guias Markdown nos dois idiomas. Edite os arquivos de origem para atualizar a documentação publicada. Para alterar a página inicial, os estilos, a compilação ou a publicação no GitHub Pages, consulte o [guia de contribuição do site](site/README.pt-BR.md).
