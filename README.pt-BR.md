# Compust

[English (US)](README.md) | [Português (Brasil)](README.pt-BR.md)

[Site do projeto e documentação](https://hashdefault.github.io/compust/pt-br/)

Compust é um **compositor independente para X11, escrito em Rust**, ainda experimental, voltado ao Xorg e ao XLibre. A proposta é construir uma alternativa pequena e compreensível ao picom, com animações suaves, desfoque de fundo e transparência.

O compositor combina as janelas dos aplicativos para formar a imagem final da área de trabalho. O Compust funciona **sobre um servidor X já em execução**, junto ao seu gerenciador de janelas. Ele não inicia nem substitui o Xorg/XLibre, não organiza as janelas e não oferece uma sessão Wayland nativa.

**Estado atual: protótipo funcional, versão 0.1.0.** O backend usa XRender e possui testes automatizados de pixels no Xvfb. Sessões com Xorg e XLibre em hardware real ainda precisam de testes da comunidade. O Compust ainda não substitui o picom em todas as suas funções, e nenhuma vantagem de desempenho sobre ele foi demonstrada. O [roteiro de desenvolvimento](docs/ROADMAP.pt-BR.md) explica o trabalho necessário para chegar lá.

## O que já funciona

A abertura e o fechamento de janelas usam uma transição de opacidade com curva smoothstep, inclusive quando uma janela é fechada durante a animação de abertura. A transparência combina o conteúdo ARGB do aplicativo, `_NET_WM_WINDOW_OPACITY` e a opacidade global configurada. Janelas translúcidas podem desfocar o conteúdo atrás delas com um filtro de caixa separável.

O Compust acompanha empilhamento, movimento, redimensionamento, formato das janelas, atualizações de conteúdo e pixmaps de papel de parede. Os pixmaps nomeados são preservados durante a animação de fechamento. A janela de composição tem região de entrada vazia, permitindo que os cliques cheguem aos aplicativos. Um compositor existente nunca é substituído automaticamente.

| Extensão ou convenção | Comportamento atual |
| --- | --- |
| Composite 0.4+ | Obrigatória: redirecionamento manual, pixmaps nomeados e janela de composição |
| Damage 1.0+ | Obrigatória: notificações de atualização; a área de trabalho parada não é redesenhada continuamente |
| Render 0.11+ | Obrigatória: composição, máscaras de opacidade e convolução quando disponível |
| XFixes 2.0+ e Shape 1.1+ | Obrigatórias: passagem de entrada e janelas com formatos não retangulares |
| Present | Opcional: apresentação por cópia, aguardando conclusão e liberação do buffer |
| RandR | Opcional: eventos de mudança da tela e recriação de buffers; hotplug físico registrado em um desktop AMD/XLibre |
| EWMH / ICCCM | Seleção do compositor, anúncio MANAGER, opacidade e descoberta do cliente por `WM_STATE` |
| Papel de parede | `_XROOTPMAP_ID`, depois `ESETROOT_PMAP_ID`; fundo escuro quando nenhum é utilizável |
| DRI3 / Sync | Apenas diagnóstico de versões; sem importação DMA-BUF nem backend com sincronização explícita |

“Suporte moderno a X11” é um objetivo incremental de compatibilidade, não uma promessa de implementar todas as extensões. A disponibilidade de Present não comprova ausência de tearing em todos os drivers. A cópia direta com XRender não é sincronizada com o intervalo vertical do monitor.

Se um envio Present for rejeitado com `BadMatch` e os buffers originais de renderização continuarem válidos, o Compust registra um aviso e usa cópia direta via XRender até reiniciar. Após uma mudança RandR, o Compust recria seus buffers sem esperar por uma apresentação pendente, porque uma reconfiguração de monitores em hardware AMD descartou os eventos de conclusão dela. Os demais erros de protocolo mantêm seus tratamentos existentes.

## Compilar e executar

Você precisa de Linux, Rust 1.95.0 (selecionado pelo arquivo de toolchain), Cargo, um linker C e um servidor X com as extensões obrigatórias. Se usa rustup, confira se `~/.cargo/bin` está no `PATH`. A conexão X11 usa a implementação Rust do `x11rb`; não é necessário instalar cabeçalhos de desenvolvimento da Xlib.

```sh
git clone https://github.com/hashdefault/compust.git
cd compust
cargo build --release --locked
./target/release/compust --help
./target/release/compust --diagnose
./target/release/compust --config compust.example.toml
```

Execute na sua sessão X11 depois de encerrar o compositor que atende aquela tela. `--diagnose` apenas consulta as extensões e pode ser executado com outro compositor ativo. Use `--display :1` para escolher outro servidor; sem essa opção, são usados `DISPLAY` e a autenticação Xauthority habitual. Encerre com Ctrl+C ou SIGTERM. O Compust não instala inicialização automática nem modifica a configuração da área de trabalho.

Para experimentar de forma isolada, inicie um servidor aninhado e execute o Compust e um aplicativo X11 nesse display:

```sh
Xephyr :99 -screen 1280x720 -ac -nolisten tcp &
DISPLAY=:99 ./target/release/compust --config compust.example.toml
# Em outro terminal:
DISPLAY=:99 xterm
```

Escolha um número de display livre. O exemplo desativa a autenticação apenas no servidor local de teste e não abre uma porta TCP. Xephyr e xterm são pacotes separados. A suíte automatizada usa Xvfb e escolhe os números de display automaticamente.

## Configuração

O arquivo de exemplo contém todas as opções. Sem `--config`, valem os padrões internos; ainda não há busca automática de arquivo nem recarga durante a execução. Campos desconhecidos e valores fora do intervalo geram erro antes da conexão com o X11.

```toml
opacity = 100
fade_ms = 180
blur_radius = 4
max_fps = 120
vsync = true
```

| Opção | Significado |
| --- | --- |
| `opacity` | Opacidade global de 0 a 100%, multiplicada pela opacidade do aplicativo |
| `fade_ms` | Duração da abertura e do fechamento em milissegundos, de 0 a 65535; zero desativa a animação |
| `blur_radius` | Raio do filtro de caixa, de 0 a 16; zero desativa o desfoque |
| `max_fps` | Limite de redesenho, de 1 a 1000; não garante essa taxa de quadros |
| `vsync` | Usa Present quando disponível; `false` seleciona cópia direta com XRender |

O desfoque é aplicado atrás de janelas translúcidas ou ARGB. Se o servidor não oferecer convolução, o Compust registra um aviso e continua sem desfoque. `max_fps` não força redesenhos quando nada muda; o loop de eventos acorda no máximo uma vez por segundo durante a inatividade para observar sinais de encerramento.

```sh
./target/release/compust --check-config --config compust.example.toml
RUST_LOG=compust=debug ./target/release/compust
```

## Testes e contribuições

Instale Xvfb (`xvfb` no Debian/Ubuntu, `xorg-server-xvfb` no Arch) e execute:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
```

Cada cenário de integração inicia seu próprio Xvfb e um processo real do Compust. Os testes observam os pixels renderizados e o protocolo, sem simular o servidor. Use `XVFB=/caminho/para/Xvfb` para indicar outro executável. `COMPUST_ARTIFACTS=artifacts cargo test --test x11` salva algumas cenas em PPM para inspeção.

Contribuições em **português brasileiro ou inglês** são bem-vindas. Comece pelo [guia de contribuição](CONTRIBUTING.pt-BR.md), pela [arquitetura](docs/ARCHITECTURE.pt-BR.md) ou pelo [roteiro](docs/ROADMAP.pt-BR.md). Relatos sobre drivers, falhas reproduzíveis, documentação e medições de desempenho também ajudam.

## Limitações atuais

O protótipo redesenha a tela inteira quando recebe dano. Para cada janela translúcida, o desfoque faz trabalho intermediário sobre a tela inteira e pode ser caro. Redesenho por regiões, descarte de áreas ocultas, backends de GPU e benchmarks comparativos ainda estão em aberto. Não há sombras, cantos arredondados, animações de movimento ou escala, regras por janela, recarga de configuração, suspensão da composição em tela cheia ou compatibilidade com arquivos do picom.

O marco planejado de **Animações de janelas** no [roteiro](docs/ROADMAP.pt-BR.md) amplia o fade existente com pop, slide, curvas e regras por janela. Seus exemplos de configuração descrevem trabalho futuro e não são aceitos pelo binário atual.

Um processo atende uma tela X; uma raiz com vários monitores é composta como uma única superfície. Agendamento para taxas de atualização diferentes, HDR/gerenciamento de cores, VRR, importação DMA-BUF, sincronização explícita e extensões exclusivas do XLibre não estão implementados ou certificados. O hotplug físico foi verificado apenas no [desktop AMD/XLibre registrado](docs/DESKTOP_TESTING.pt-BR.md#sessão-registrada-em-hardware-2026-10-03). Wayland nativo está fora do escopo atual.

## Contexto e licença

O projeto se inspira no modelo de compositor independente do [picom](https://github.com/yshui/picom), cuja origem inclui o Compton. O picom já oferece animações e efeitos; a contribuição pretendida pelo Compust é uma implementação acessível em Rust, com melhorias medidas ao longo do desenvolvimento. Este repositório contém uma implementação nova, não uma tradução do código do picom.

Referências: [x11rb](https://docs.rs/x11rb/0.13.2/x11rb/), [compositores na especificação EWMH](https://specifications.freedesktop.org/wm/latest/ar01s08.html) e [XLibre](https://github.com/X11Libre/xserver). Licença [MIT](LICENSE).
