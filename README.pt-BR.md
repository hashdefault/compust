# Compust

[English (US)](README.md) | [Português (Brasil)](README.pt-BR.md)

[Site do projeto e documentação](https://hashdefault.github.io/compust/pt-br/)

Compust é um **compositor independente para X11, escrito em Rust**, ainda experimental, voltado ao Xorg e ao XLibre. A proposta é construir uma alternativa pequena e compreensível ao picom, com animações suaves, desfoque de fundo e transparência.

O compositor combina as janelas dos aplicativos para formar a imagem final da área de trabalho. O Compust funciona **sobre um servidor X já em execução**, junto ao seu gerenciador de janelas. Ele não inicia nem substitui o Xorg/XLibre, não organiza as janelas e não oferece uma sessão Wayland nativa.

**Estado atual: terceira beta, versão 0.2.0-beta.3, para testes controlados.** O backend XRender possui testes automatizados de pixels no Xvfb e validação registrada em duas máquinas: um desktop AMD com XLibre e Xmonad, e um laptop Intel com Xorg e Xmonad, Openbox e i3; outros drivers, servidores e gerenciadores de janelas ainda precisam de testes da comunidade. O Compust ainda não substitui o picom em todas as suas funções, e nenhuma vantagem de desempenho sobre ele foi demonstrada. O [guia da beta](docs/BETA.pt-BR.md) explica como instalar, voltar ao compositor anterior e relatar problemas; o [roteiro de desenvolvimento](docs/ROADMAP.pt-BR.md) lista o trabalho restante.

## O que já funciona

A abertura e o fechamento de janelas usam uma transição de opacidade com curva smoothstep, inclusive quando uma janela é fechada durante a animação de abertura. A transparência combina o conteúdo ARGB do aplicativo, `_NET_WM_WINDOW_OPACITY` e a opacidade global configurada. Janelas translúcidas podem desfocar o conteúdo atrás delas. O desfoque reduz repetidamente pela metade a área atrás da janela com amostragem bilinear e depois a amplia de volta, operações que servidores com aceleração por GPU mantêm na GPU. [Regras por janela](#regras-por-janela) definem a opacidade, o desfoque e a duração do fade das janelas escolhidas por classe, tipo ou título.

O Compust acompanha empilhamento, movimento, redimensionamento, formato das janelas, atualizações de conteúdo e pixmaps de papel de parede. Os pixmaps nomeados são preservados durante a animação de fechamento. A janela de composição tem região de entrada vazia, permitindo que os cliques cheguem aos aplicativos. Um compositor existente nunca é substituído automaticamente.

| Extensão ou convenção | Comportamento atual |
| --- | --- |
| Composite 0.4+ | Obrigatória: redirecionamento manual, pixmaps nomeados e janela de composição |
| Damage 1.0+ | Obrigatória: notificações de atualização; a área de trabalho parada não é redesenhada continuamente |
| Render 0.11+ | Obrigatória: composição, máscaras de opacidade e transformações; a filtragem bilinear habilita o desfoque |
| XFixes 2.0+ e Shape 1.1+ | Obrigatórias: passagem de entrada e janelas com formatos não retangulares |
| Present | Opcional: apresentação por cópia, aguardando conclusão e liberação do buffer |
| RandR | Opcional: eventos de mudança da tela e recriação de buffers; hotplug físico registrado em um desktop AMD/XLibre e em um laptop Intel/Xorg |
| EWMH / ICCCM | Seleção do compositor, anúncio MANAGER, opacidade, descoberta do cliente por `WM_STATE` e regras que comparam `WM_CLASS`, `_NET_WM_WINDOW_TYPE` (com `WM_TRANSIENT_FOR` para o tipo padrão) e `_NET_WM_NAME` ou `WM_NAME` |
| Papel de parede | `_XROOTPMAP_ID`, depois `ESETROOT_PMAP_ID`; fundo escuro quando nenhum é utilizável |
| DRI3 1.2 e Sync 3.1 | Opcionais: o renderizador de GPU compartilha o buffer de fundo e os pixmaps das janelas e do papel de parede por DRI3, e uma fence do Sync faz o servidor enviar seu trabalho de GPU antes de cada quadro; sem sincronização explícita |

“Suporte moderno a X11” é um objetivo incremental de compatibilidade, não uma promessa de implementar todas as extensões. A disponibilidade de Present não comprova ausência de tearing em todos os drivers. A cópia direta com XRender não é sincronizada com o intervalo vertical do monitor.

Se um envio Present for rejeitado com `BadMatch` e os buffers originais de renderização continuarem válidos, o Compust registra um aviso e usa cópia direta via XRender até reiniciar. Após uma mudança RandR, o Compust recria seus buffers sem esperar por uma apresentação pendente, porque uma reconfiguração de monitores em hardware AMD descartou os eventos de conclusão dela. Se o Present não informar nada sobre um envio em um segundo, o Compust também recria seus buffers e repinta, para que uma notificação perdida não congele a tela. Os demais erros de protocolo mantêm seus tratamentos existentes.

## Compilar e executar

Você precisa de Linux, Rust 1.95.0 (selecionado pelo arquivo de toolchain), Cargo, um linker C e um servidor X com as extensões obrigatórias. Se usa rustup, confira se `~/.cargo/bin` está no `PATH`. A conexão X11 usa a implementação Rust do `x11rb`; não é necessário instalar cabeçalhos de desenvolvimento da Xlib. Quem for testar a beta pode baixar um binário verificado, como explica o [guia da beta](docs/BETA.pt-BR.md).

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

O arquivo de exemplo contém todas as opções. Sem `--config`, o Compust lê o primeiro `compust/compust.toml` que encontrar em `$XDG_CONFIG_HOME` (por padrão `~/.config`) e depois em cada diretório de `$XDG_CONFIG_DIRS` (por padrão `/etc/xdg`); se não houver arquivo, valem os padrões internos. Campos desconhecidos e valores fora do intervalo geram erro antes da conexão com o X11.

Envie SIGUSR1 para recarregar a configuração sem reiniciar, por exemplo com `pkill -USR1 -x compust`. A recarga lê o mesmo arquivo que uma reinicialização leria. Se esse arquivo não puder ser lido ou for inválido, o Compust registra um aviso e mantém as opções atuais. `opacity`, `max_fps` e a opacidade e o desfoque dados por regras valem a partir do próximo quadro, também para janelas já abertas. Um novo `fade_ms`, global ou em uma regra, vale para toda abertura e todo fechamento iniciados depois, inclusive de janelas já abertas; fades em andamento terminam com a duração anterior. Uma mudança em `blur_radius`, `vsync` ou `backend` substitui o renderizador assim que o quadro em apresentação termina.

```toml
opacity = 100
fade_ms = 180
blur_radius = 4
max_fps = 120
vsync = true
backend = "xrender"
```

| Opção | Significado |
| --- | --- |
| `opacity` | Opacidade global de 0 a 100%, multiplicada pela opacidade do aplicativo |
| `fade_ms` | Duração da abertura e do fechamento em milissegundos, de 0 a 65535; zero desativa a animação |
| `blur_radius` | Raio aproximado do desfoque em pixels, de 0 a 16, arredondado para 2, 4, 8 ou 16; zero desativa o desfoque |
| `max_fps` | Limite de redesenho, de 1 a 1000; não garante essa taxa de quadros |
| `vsync` | Usa Present quando disponível; `false` seleciona cópia direta com XRender |
| `backend` | `"xrender"` desenha pelo servidor X; `"gl"` desenha com OpenGL ES na GPU do servidor e volta ao XRender com um aviso onde não puder |

O desfoque é aplicado atrás de janelas translúcidas ou ARGB, com a força com que cada pixel da janela é opaco: a margem transparente de sombra em volta do menu de um navegador quase não recebe desfoque, e o desfoque surge e some com a janela. Se o servidor não oferecer filtragem bilinear, o Compust registra um aviso e continua sem desfoque. `max_fps` não força redesenhos quando nada muda; o loop de eventos acorda no máximo uma vez por segundo durante a inatividade para observar sinais de encerramento e de recarga.

### Regras por janela

Cada tabela `[[rules]]` escolhe janelas e muda opções para elas. As regras vêm depois das opções globais, porque o TOML coloca toda tabela depois das chaves simples.

```toml
# Terminais com 90% de opacidade.
[[rules]]
wm_class = "Alacritty"
opacity = 90

# Dicas de ferramenta sem desfoque atrás e sem fades.
[[rules]]
window_type = "tooltip"
blur = false
fade_ms = 0
```

| Campo | Significado |
| --- | --- |
| `wm_class` | Escolhe janelas cuja classe de recurso, a segunda string de `WM_CLASS`, é este texto |
| `window_type` | Escolhe janelas deste tipo EWMH: `desktop`, `dock`, `toolbar`, `menu`, `utility`, `splash`, `dialog`, `dropdown_menu`, `popup_menu`, `tooltip`, `notification`, `combo`, `dnd` ou `normal` |
| `name` | Escolhe janelas cujo título, `_NET_WM_NAME` ou, na falta dele, `WM_NAME`, é este texto |
| `opacity` | Porcentagem de opacidade, de 0 a 100, usada no lugar da `opacity` global e multiplicada pela opacidade do aplicativo |
| `blur` | `false` mantém nítido o conteúdo atrás da janela; `true` o desfoca como por padrão, enquanto `blur_radius` for maior que zero |
| `fade_ms` | Duração da abertura e do fechamento em milissegundos, de 0 a 65535, usada no lugar do `fade_ms` global |

Uma regra precisa de ao menos um dos três primeiros campos, que escolhem janelas, e de ao menos um dos três últimos, que ela define. O texto precisa ser idêntico, inclusive em maiúsculas e minúsculas, e a janela precisa corresponder a todos os campos pelos quais a regra escolhe. Cada opção vem da primeira regra correspondente que a define, então regras específicas vêm antes das amplas; assim, `true` em uma regra mantém o desfoque de janelas para as quais uma regra posterior o desliga. Rode `xprop` e clique em uma janela para ver seus `WM_CLASS`, `_NET_WM_WINDOW_TYPE` e `_NET_WM_NAME`.

O Compust lê essas propriedades da janela do aplicativo, dentro da moldura do gerenciador de janelas, e as lê de novo quando mudam, como quando um título muda. Uma janela que não declara nenhum tipo conhecido pelo Compust é `dialog` quando é transitória para outra janela e o gerenciador de janelas a controla, e `normal` nos demais casos, como determina a EWMH. Uma propriedade ausente ou malformada não corresponde a nenhum texto. Uma janela em fechamento mantém as regras que tinha enquanto o fade termina.

```sh
./target/release/compust --check-config --config compust.example.toml
./target/release/compust --check-config  # informa qual arquivo seria lido
RUST_LOG=compust=debug ./target/release/compust
```

## Testes e contribuições

Instale Xvfb (`xvfb` no Debian/Ubuntu, `xorg-server-xvfb` no Arch) e execute:

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace
```

Cada cenário de integração inicia seu próprio Xvfb e um processo real do Compust. Os testes observam os pixels renderizados e o protocolo, sem simular o servidor. Use `XVFB=/caminho/para/Xvfb` para indicar outro executável. `COMPUST_ARTIFACTS=artifacts cargo test --test x11` salva algumas cenas em PPM para inspeção. Os testes do crate de GPU desenham no dispositivo EGL de software do Mesa e são pulados sem ele; `COMPUST_GPU_TESTS=1`, como na CI, transforma isso em falha. `COMPUST_GPU_DISPLAY=:0 cargo test --test gpu` confere o compartilhamento por DRI3 com a GPU de um servidor real usando apenas pixmaps fora da tela.

Contribuições em **português brasileiro ou inglês** são bem-vindas. Comece pelo [guia de contribuição](CONTRIBUTING.pt-BR.md), pela [arquitetura](docs/ARCHITECTURE.pt-BR.md) ou pelo [roteiro](docs/ROADMAP.pt-BR.md). Relatos sobre drivers, falhas reproduzíveis, documentação e medições de desempenho também ajudam.

## Limitações atuais

O protótipo repinta apenas a área da tela que mudou, e cada janela desfocada guarda seu fundo desfocado até que algo abaixo dela mude, o que a desfoca de novo em toda a sua área de alcance. No [desktop AMD/XLibre registrado](docs/DESKTOP_TESTING.pt-BR.md#sessão-registrada-do-desfoque-em-pirâmide-2026-10-03), uma janela translúcida em tela cheia com desfoque manteve 60 quadros por segundo enquanto o Xorg usava cerca de 4% de um núcleo. Janelas escondidas atrás de janelas opacas não são pintadas. O renderizador de GPU opcional ([registrado em um desktop](docs/DESKTOP_TESTING.pt-BR.md#renderizador-de-gpu-registrado-2026-10-04)) desenha os mesmos quadros que o XRender com diferença de até dois níveis de cor, com CPU total parecida ali, maior quando janelas são redimensionadas, e cerca de 62 MiB a mais de memória para o driver GL. O [marco de renderização](docs/ROADMAP.pt-BR.md#trabalho-de-renderização-quatro-etapas-concluídas-em-um-desktop) mediu esses custos; seus [primeiros registros](docs/DESKTOP_TESTING.pt-BR.md#cenas-de-benchmark-registradas-2026-10-03) comparam o Compust com o picom em uma máquina, antes das mudanças na repintura. Não há sombras, cantos arredondados, animações de movimento ou escala, suspensão da composição em tela cheia ou compatibilidade com arquivos do picom; o [marco 1.0](docs/ROADMAP.pt-BR.md#10-versão-estável) acrescenta sombras e a suspensão da composição em tela cheia.

O marco planejado de **Animações de janelas** no [roteiro](docs/ROADMAP.pt-BR.md) amplia o fade existente com pop, slide e curvas, escolhidos por janela. Seus exemplos de configuração descrevem trabalho futuro e não são aceitos pelo binário atual.

Um processo atende uma tela X; uma raiz com vários monitores é composta como uma única superfície. Agendamento para taxas de atualização diferentes, HDR/gerenciamento de cores, VRR, importação DMA-BUF, sincronização explícita e extensões exclusivas do XLibre não estão implementados ou certificados. O hotplug físico foi verificado apenas no [desktop AMD/XLibre registrado](docs/DESKTOP_TESTING.pt-BR.md#sessão-registrada-em-hardware-2026-10-03) e no [laptop Intel/Xorg registrado](docs/DESKTOP_TESTING.pt-BR.md#sessão-registrada-de-hotplug-em-intelxorg-2026-10-03). Wayland nativo está fora do escopo atual.

## Contexto e licença

O projeto se inspira no modelo de compositor independente do [picom](https://github.com/yshui/picom), cuja origem inclui o Compton. O picom já oferece animações e efeitos; a contribuição pretendida pelo Compust é uma implementação acessível em Rust, com melhorias medidas ao longo do desenvolvimento. Este repositório contém uma implementação nova, não uma tradução do código do picom.

Referências: [x11rb](https://docs.rs/x11rb/0.13.2/x11rb/), [compositores na especificação EWMH](https://specifications.freedesktop.org/wm/latest/ar01s08.html) e [XLibre](https://github.com/X11Libre/xserver). Licença [MIT](LICENSE).
