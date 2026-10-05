# Compust

[English (US)](README.md) | [Português (Brasil)](README.pt-BR.md)

[Site do projeto e documentação](https://hashdefault.github.io/compust/pt-br/)

Compust é um **compositor independente para X11, escrito em Rust**, ainda experimental, voltado ao Xorg e ao XLibre. A proposta é construir uma alternativa pequena e compreensível ao picom, com animações suaves, desfoque de fundo e transparência.

O compositor combina as janelas dos aplicativos para formar a imagem final da área de trabalho. O Compust funciona **sobre um servidor X já em execução**, junto ao seu gerenciador de janelas. Ele não inicia nem substitui o Xorg/XLibre, não organiza as janelas e não oferece uma sessão Wayland nativa.

**Estado atual: quarta beta, versão 0.3.0-beta.1, para testes controlados.** Testes automatizados de pixels rodam no Xvfb, e há sessões registradas em três máquinas. Um desktop AMD com Radeon RX 9060 XT e XLibre é a única que rodou esta beta, com Xmonad, Openbox, i3, bspwm e dwm; outro desktop AMD com XLibre e um laptop Intel com Xorg foram registrados com as betas anteriores. Outros drivers, servidores e gerenciadores de janelas ainda precisam de testes da comunidade. Seus recursos e desempenho são documentados apenas para as cargas e os ambientes registrados. O [guia da beta](docs/BETA.pt-BR.md) explica como instalar, voltar ao compositor anterior e relatar problemas. Os [marcos](#marcos-e-ambientes-verificados) abaixo mostram o que está pronto, e o [marco 1.0](docs/ROADMAP.pt-BR.md#10-versão-estável) do [roteiro de desenvolvimento](docs/ROADMAP.pt-BR.md) define o que uma versão estável ainda exige.

## Marcos e ambientes verificados

Cada linha leva ao seu registro.

| Marco | Situação | O que estabeleceu |
| --- | --- | --- |
| [Base 0.1](docs/ROADMAP.pt-BR.md#base-01-implementada) | Concluído | Composição com XRender, fades, transparência, desfoque, formatos e empilhamento, conferidos por testes de pixels em um servidor X real |
| [Primeira beta](docs/ROADMAP.pt-BR.md#primeira-beta-quatro-etapas) | Publicada como 0.2.0-beta.1, beta.2 e beta.3 | Quatro critérios cumpridos para o escopo declarado: estabilidade das janelas, monitores e recursos, desktops reais e um arquivo de versão verificado |
| [Recarga de configuração e regras](docs/ROADMAP.pt-BR.md#uso-cotidiano) | Publicado na 0.3.0-beta.1 | Descoberta da configuração, recarga com SIGUSR1, regras por janela e desfoque ponderado pela opacidade de cada pixel |
| [Trabalho de renderização](docs/ROADMAP.pt-BR.md#trabalho-de-renderização-quatro-etapas-concluídas-em-um-desktop) | Publicado na 0.3.0-beta.1; medido em um desktop | Cenas independentes de benchmark do Compust, empilhamento acompanhado sem consultas à árvore, repintura por regiões, reaproveitamento do desfoque e oclusão |
| [Renderizador de GPU](docs/ROADMAP.pt-BR.md#backend-e-expansão-do-protocolo) | Opcional desde a 0.3.0-beta.1; registrado em um desktop | OpenGL ES por DRI3, desenhando os mesmos quadros que o XRender com diferença de até dois níveis de cor; o XRender continua como padrão e como retorno |
| [Versão estável 1.0](docs/ROADMAP.pt-BR.md#10-versão-estável) | Marco atual; seus recursos estão prontos no `main`: [sombras](docs/ROADMAP.pt-BR.md#etapa-1-da-10-sombras), [suspensão da composição em tela cheia](docs/ROADMAP.pt-BR.md#etapa-1-da-10-suspensão-da-composição-em-tela-cheia) e [regras pelo foco](docs/ROADMAP.pt-BR.md#etapa-1-da-10-regras-pelo-foco) | NVIDIA e mais desktops, medições independentes de desempenho, testes de uso prolongado, testadores externos e empacotamento |

| Ambiente | O que foi registrado |
| --- | --- |
| Xvfb na CI, a cada push | 123 testes X11 que rodam um processo real do Compust em um servidor real e conferem pixels e protocolo, além de 45 testes unitários, 6 de linha de comando e os 9 testes do crate de GPU no dispositivo de software do Mesa |
| Desktop AMD Ryzen 5 5600GT (Radeon Vega): XLibre 25.1.9 com Xmonad 0.18.1, um e dois monitores 1920×1080; Xorg 21.1.24 com i3 4.25.1, um monitor | No XLibre: [cenários de desktop](docs/DESKTOP_TESTING.pt-BR.md#sessão-de-desktop-registrada-em-hardware-2026-10-03) com fades, translucidez e [desfoque](docs/DESKTOP_TESTING.pt-BR.md#sessão-registrada-do-desfoque-em-pirâmide-2026-10-03); [mudanças de modo e de layout, e os dois cabos desconectados e reconectados](docs/DESKTOP_TESTING.pt-BR.md#sessão-registrada-em-hardware-2026-10-03). No Xorg: [cenários de desktop com o `main`, e sombras, regras de foco e suspensão em tela cheia nos dois pintores](docs/DESKTOP_TESTING.pt-BR.md#sessões-xorg-registradas-no-desktop-com-radeon-vega-2026-10-05) |
| Laptop Intel Core i3-1005G1 (Iris Plus): Xorg 21.1.11, com Xmonad 0.17.2, Openbox 3.6.1 e i3 4.23 | Cenários de desktop com [Xmonad](docs/DESKTOP_TESTING.pt-BR.md#sessões-registradas-em-intelxorg-2026-10-03), [Openbox](docs/DESKTOP_TESTING.pt-BR.md#sessões-registradas-com-openbox-2026-10-03) e [i3](docs/DESKTOP_TESTING.pt-BR.md#sessões-registradas-com-i3-2026-10-03); um [monitor externo desconectado e reconectado](docs/DESKTOP_TESTING.pt-BR.md#sessão-registrada-de-hotplug-em-intelxorg-2026-10-03); [suspensão e retomada](docs/DESKTOP_TESTING.pt-BR.md#suspensão-e-retomada-registradas-2026-10-03) |
| AMD Ryzen 5 5600X com Radeon RX 9060 XT: XLibre 25.1.9, com Xmonad 0.18.1, Openbox 3.6.1, i3 4.25.1, bspwm 0.9.12 e dwm 6.8, um e dois monitores 1920×1080 | [Cenários de desktop](docs/DESKTOP_TESTING.pt-BR.md#sessões-de-desktop-registradas-na-rx-9060-xt-2026-10-04) com o código da 0.3.0-beta.1 em Xmonad, Openbox, i3 e bspwm, com fades, translucidez e desfoque. Com o dwm: [recargas de configuração](docs/DESKTOP_TESTING.pt-BR.md#sessão-registrada-de-recarga-2026-10-03); [cenas de benchmark do Compust](docs/DESKTOP_TESTING.pt-BR.md#cenas-de-benchmark-registradas-2026-10-03); [repintura por regiões](docs/DESKTOP_TESTING.pt-BR.md#repintura-por-regiões-registrada-2026-10-04), [reaproveitamento do desfoque](docs/DESKTOP_TESTING.pt-BR.md#reaproveitamento-do-desfoque-registrado-2026-10-04), [oclusão](docs/DESKTOP_TESTING.pt-BR.md#oclusão-registrada-2026-10-04) e o [renderizador de GPU](docs/DESKTOP_TESTING.pt-BR.md#renderizador-de-gpu-registrado-2026-10-04) |
| Xephyr aninhado no Xvfb: Xorg 21.1.24 e XLibre 25.1.9, com Xmonad 0.18.1 | [Cenários de desktop](docs/DESKTOP_TESTING.pt-BR.md#medição-registrada-2026-10-03) com Present, com cópia direta via XRender e com [efeitos](docs/DESKTOP_TESTING.pt-BR.md#medição-registrada-com-efeitos-2026-10-03); apenas o comportamento do protocolo, sem driver nem monitor |

A maioria das sessões em hardware usa as janelas sintéticas do probe em vez de aplicativos. O laptop Intel e as sessões XLibre do desktop com Radeon Vega foram registrados antes do trabalho de renderização; desde então, esse desktop foi [registrado no Xorg](docs/DESKTOP_TESTING.pt-BR.md#sessões-xorg-registradas-no-desktop-com-radeon-vega-2026-10-05) com o `main`. [Sombras, regras de foco controlado e suspensão em tela cheia](docs/DESKTOP_TESTING.pt-BR.md#sombras-foco-e-suspensão-em-tela-cheia-registrados-2026-10-04) agora têm um registro de cenas fixas nos dois pintores na RX 9060 XT sem gerenciador de janelas; os outros casos de regras têm testes automatizados. [Monitores com taxas diferentes](docs/DESKTOP_TESTING.pt-BR.md#monitores-com-taxas-diferentes-registrados-2026-10-04) foram conferidos com HDMI-1 a 60 Hz e DP-2 a 180 Hz em XRender e GL. O desfoque ponderado também foi conferido visualmente nos menus do Brave. Drivers NVIDIA, XLibre com Intel, outros gerenciadores de janelas, mais de dois monitores e HDR não foram testados. A [matriz de compatibilidade](docs/ROADMAP.pt-BR.md#matriz-de-compatibilidade) traz as versões e os limites de cada registro.

## O que já funciona

A abertura e o fechamento de janelas usam uma transição de opacidade com curva smoothstep, inclusive quando uma janela é fechada durante a animação de abertura. A transparência combina o conteúdo ARGB do aplicativo, `_NET_WM_WINDOW_OPACITY` e a opacidade global configurada. Janelas translúcidas podem desfocar o conteúdo atrás delas. O desfoque reduz repetidamente pela metade a área atrás da janela com amostragem bilinear e depois a amplia de volta, operações que servidores com aceleração por GPU mantêm na GPU. As janelas podem projetar [sombras](#sombras) suaves, que ficam desligadas até que `shadow_radius` seja definido. [Regras por janela](#regras-por-janela) definem a opacidade, o desfoque, a duração do fade e a sombra das janelas escolhidas por classe, tipo, título ou foco.

O Compust acompanha empilhamento, movimento, redimensionamento, formato das janelas, atualizações de conteúdo e pixmaps de papel de parede. Os pixmaps nomeados são preservados durante a animação de fechamento. A janela de composição tem região de entrada vazia, permitindo que os cliques cheguem aos aplicativos. Um compositor existente nunca é substituído automaticamente.

| Extensão ou convenção | Comportamento atual |
| --- | --- |
| Composite 0.4+ | Obrigatória: redirecionamento manual, pixmaps nomeados e janela de composição |
| Damage 1.0+ | Obrigatória: notificações de atualização; a área de trabalho parada não é redesenhada continuamente |
| Render 0.11+ | Obrigatória: composição, máscaras de opacidade e transformações; a filtragem bilinear habilita o desfoque |
| XFixes 2.0+ e Shape 1.1+ | Obrigatórias: passagem de entrada e janelas com formatos não retangulares |
| Present | Opcional: apresentação por cópia, aguardando conclusão e liberação do buffer |
| RandR | Opcional: eventos de mudança da tela e recriação de buffers; hotplug físico registrado em um desktop AMD/XLibre e em um laptop Intel/Xorg |
| EWMH / ICCCM | Seleção do compositor, anúncio MANAGER, opacidade, descoberta do cliente por `WM_STATE` e regras que comparam `WM_CLASS`, `_NET_WM_WINDOW_TYPE` (com `WM_TRANSIENT_FOR` para o tipo padrão) e `_NET_WM_NAME` ou `WM_NAME`, ou que acompanham o `_NET_ACTIVE_WINDOW` da raiz; `_GTK_FRAME_EXTENTS` indica janelas que desenham a própria sombra |
| Papel de parede | `_XROOTPMAP_ID`, depois `ESETROOT_PMAP_ID`; fundo escuro quando nenhum é utilizável |
| DRI3 1.2 e Sync 3.1 | Opcionais: o renderizador de GPU compartilha o buffer de fundo e os pixmaps das janelas e do papel de parede por DRI3, e uma fence do Sync faz o servidor enviar seu trabalho de GPU antes de cada quadro; sem sincronização explícita |

“Suporte moderno a X11” é um objetivo incremental de compatibilidade, não uma promessa de implementar todas as extensões. A disponibilidade de Present não comprova ausência de tearing em todos os drivers. A cópia direta com XRender não é sincronizada com o intervalo vertical do monitor.

Se um envio Present for rejeitado com `BadMatch` e os buffers originais de renderização continuarem válidos, o Compust registra um aviso e usa cópia direta via XRender até reiniciar. Após uma mudança RandR, o Compust recria seus buffers sem esperar por uma apresentação pendente, porque uma reconfiguração de monitores em hardware AMD descartou os eventos de conclusão dela. Se o Present não informar nada sobre um envio em um segundo, o Compust também recria seus buffers e repinta, para que uma notificação perdida não congele a tela. O servidor ainda pode exibir depois um envio dos buffers substituídos, até mesmo após o quadro novo; quando ele informa um, o Compust exibe de novo seu quadro atual. Os demais erros de protocolo mantêm seus tratamentos existentes.

## Instalação

O [Open Build Service](https://build.opensuse.org/package/show/home:hashdefault/compust) gera pacotes x86_64 para Debian, Ubuntu, Linux Mint, Fedora e openSUSE Tumbleweed a partir de um snapshot testado da `main` posterior à 0.3.0-beta.1, e roda a suíte de testes em cada build. As [notas de empacotamento](packaging/obs/README.md) dizem qual é o snapshot. O pacote instala apenas o programa `compust` e a documentação dele: nenhuma entrada de inicialização automática e nenhuma alteração na configuração do seu desktop.

### Arch Linux e derivados

Ainda não há pacote para Arch, então compile a partir do código-fonte. Os mesmos comandos valem para derivados que usam `pacman`, como CachyOS, EndeavourOS e Manjaro.

```sh
sudo pacman -S --needed git rust
git clone https://github.com/hashdefault/compust.git
cd compust
cargo build --release --locked
sudo install -Dm755 target/release/compust /usr/local/bin/compust
```

O pacote `rust` do Arch é mais novo que o 1.95 exigido. Se você usa rustup em vez desse pacote, o Cargo usa o toolchain 1.95.0 fixado pelo repositório. Para atualizar depois, rode `git pull` e repita os dois últimos comandos.

### Debian, Ubuntu e Linux Mint

Um único repositório serve aos três. O pacote é gerado no Debian 13 e só precisa da glibc 2.34; ele foi instalado no Debian 13, no Ubuntu 22.04 e 24.04 e no Linux Mint 22.

```sh
sudo apt install curl gnupg
sudo install -d -m 0755 /etc/apt/keyrings
curl -fsSL https://download.opensuse.org/repositories/home:hashdefault/Debian_13/Release.key | gpg --dearmor | sudo tee /etc/apt/keyrings/compust.gpg > /dev/null
echo 'deb [signed-by=/etc/apt/keyrings/compust.gpg] https://download.opensuse.org/repositories/home:/hashdefault/Debian_13/ /' | sudo tee /etc/apt/sources.list.d/compust.list
sudo apt update
sudo apt install compust
```

O repositório se chama `Debian_13` também no Ubuntu e no Mint.

### Fedora e derivados

Há repositórios para o Fedora 43 e o 44. `rpm -E %fedora` mostra o número da sua versão, então o comando escolhe o repositório correspondente. Derivados que usam `dnf` e se baseiam nessas versões devem funcionar da mesma forma; eles não foram testados.

```sh
sudo dnf config-manager addrepo --from-repofile="https://download.opensuse.org/repositories/home:/hashdefault/Fedora_$(rpm -E %fedora)/home:hashdefault.repo"
sudo dnf install compust
```

### openSUSE Tumbleweed

```sh
sudo zypper addrepo https://download.opensuse.org/repositories/home:/hashdefault/openSUSE_Tumbleweed/home:hashdefault.repo
sudo zypper refresh
sudo zypper install compust
```

### Depois de instalar

Confira o servidor e inicie o Compust na sua sessão X11, depois de encerrar o compositor que já atende aquela tela:

```sh
compust --diagnose
compust
```

Para partir da configuração de exemplo, copie `compust.example.toml` para `~/.config/compust/compust.toml`. O arquivo está no repositório, e os pacotes o instalam em `/usr/share/doc`; `dpkg -L compust` ou `rpm -ql compust` mostra onde. A seção [Configuração](#configuração) descreve as opções, e o [guia da beta](docs/BETA.pt-BR.md) explica como voltar ao compositor anterior e relatar problemas.

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

O Compust recarrega a configuração sem reiniciar quando o arquivo é salvo, depois que ele fica um décimo de segundo sem mudar, e ao receber SIGUSR1, por exemplo com `pkill -USR1 -x compust`; a 0.3.0-beta.1 recarrega apenas com SIGUSR1. Ele observa os diretórios dos arquivos que leria e o arquivo para o qual um link simbólico aponta, então editores que salvam substituindo o arquivo também o recarregam. Um diretório de configuração criado depois que o Compust inicia é lido com SIGUSR1 ou numa reinicialização. A recarga lê o mesmo arquivo que uma reinicialização leria. Se esse arquivo não puder ser lido ou for inválido, o Compust registra um aviso e mantém as opções atuais. `opacity`, `max_fps`, `unredirect_fullscreen`, as opções de sombra e a opacidade, o desfoque e a sombra dados por regras valem a partir do próximo quadro, também para janelas já abertas. Um novo `fade_ms`, global ou em uma regra, vale para toda abertura e todo fechamento iniciados depois, inclusive de janelas já abertas; fades em andamento terminam com a duração anterior. Uma mudança em `blur_radius`, `vsync` ou `backend` substitui o renderizador assim que o quadro em apresentação termina.

```toml
opacity = 100
fade_ms = 180
blur_radius = 4
shadow_radius = 0
shadow_offset_x = 0
shadow_offset_y = 0
shadow_opacity = 50
max_fps = 120
vsync = true
backend = "xrender"
unredirect_fullscreen = false
```

| Opção | Significado |
| --- | --- |
| `opacity` | Opacidade global de 0 a 100%, multiplicada pela opacidade do aplicativo |
| `fade_ms` | Duração da abertura e do fechamento em milissegundos, de 0 a 65535; zero desativa a animação |
| `blur_radius` | Raio aproximado do desfoque em pixels, de 0 a 16, arredondado para 2, 4, 8 ou 16; zero desativa o desfoque |
| `shadow_radius` | Até onde a sombra se espalha além da janela, em pixels, de 0 a 64; zero, o padrão, não desenha sombras |
| `shadow_offset_x`, `shadow_offset_y` | Onde a sombra fica em relação à janela, em pixels, de −64 a 64: para a direita e para baixo ou, com valores negativos, para a esquerda e para cima |
| `shadow_opacity` | Quão escura a sombra é no ponto mais escuro, em porcentagem, de 0 a 100, multiplicada pela opacidade da janela |
| `max_fps` | Limite de redesenho, de 1 a 1000; não garante essa taxa de quadros |
| `vsync` | Usa Present quando disponível; `false` seleciona cópia direta com XRender |
| `unredirect_fullscreen` | `true` suspende a composição enquanto uma janela opaca cobre a tela inteira; `false`, o padrão, compõe sempre |
| `backend` | `"xrender"` desenha pelo servidor X; `"gl"` desenha com OpenGL ES na GPU do servidor e volta ao XRender com um aviso onde não puder |

O desfoque é aplicado atrás de janelas translúcidas ou ARGB, com a força com que cada pixel da janela é opaco: a margem transparente de sombra em volta do menu de um navegador quase não recebe desfoque, e o desfoque surge e some com a janela. Se o servidor não oferecer filtragem bilinear, o Compust registra um aviso e continua sem desfoque. `max_fps` não força redesenhos quando nada muda; o loop de eventos acorda no máximo uma vez por segundo durante a inatividade para observar sinais de encerramento e de recarga.

### Sombras

As sombras estão no `main` e entram na próxima beta; a 0.3.0-beta.1 rejeita essas opções. Com `shadow_radius` acima de zero, as janelas projetam uma sombra preta: o retângulo da janela, deslocado pelo offset e desfocado até sumir ao longo do raio. A sombra fica em volta da janela, nunca embaixo dela, de modo que uma janela translúcida não escurece por causa da própria sombra, e ela surge e some com a janela.

Uma janela projeta sombra quando seu tipo é `normal`, `dialog`, `utility`, `splash` ou `toolbar`. Desktops, docks, menus, dicas de ferramenta, notificações, caixas de combinação e ícones de arrastar não projetam. Também não projeta a janela que não declara tipo e que o gerenciador de janelas não controla, como uma barra de status ou o menu de um toolkit antigo, nem a que declara em `_GTK_FRAME_EXTENTS` margens para uma sombra própria, como fazem as janelas GTK com decoração do lado do cliente. `shadow` em uma [regra](#regras-por-janela) decide para as janelas que ela escolhe, nos dois sentidos. Uma janela com formato não retangular nunca projeta sombra, porque a sombra seria a do seu retângulo envolvente.

Em um gerenciador de janelas tiling, a sombra de cada janela cai sobre as vizinhas. Uma regra com `window_type = "normal"` e `shadow = false` deixa as sombras para diálogos e outras janelas flutuantes que declaram seu tipo.

### Suspensão da composição em tela cheia

Com `unredirect_fullscreen = true`, o Compust para de compor enquanto uma janela esconde todo o resto: a janela do topo, quando é opaca, não tem formato e cobre a tela inteira. As janelas passam a desenhar direto na tela, como sem compositor, o que poupa a um jogo ou vídeo em tela cheia a cópia feita pelo Compust. A composição volta assim que isso deixa de valer: outra janela aparece acima dela, como um menu ou uma notificação, ou ela se move, encolhe, fica translúcida ou fecha. Sombras, desfoque e fades voltam junto.

Um único overlay cobre todos os monitores; por isso, com vários monitores, uma janela que ocupa um deles não cobre a tela e continua composta. Como as sombras, esta opção está no `main` e entra na próxima beta.

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

# Notificações com sombra, que o tipo delas deixaria de fora.
[[rules]]
window_type = "notification"
shadow = true

# Janelas que não são a ativa com 85% de opacidade.
[[rules]]
window_type = "normal"
focused = false
opacity = 85
```

| Campo | Significado |
| --- | --- |
| `wm_class` | Escolhe janelas cuja classe de recurso, a segunda string de `WM_CLASS`, é este texto |
| `window_type` | Escolhe janelas deste tipo EWMH: `desktop`, `dock`, `toolbar`, `menu`, `utility`, `splash`, `dialog`, `dropdown_menu`, `popup_menu`, `tooltip`, `notification`, `combo`, `dnd` ou `normal` |
| `name` | Escolhe janelas cujo título, `_NET_WM_NAME` ou, na falta dele, `WM_NAME`, é este texto |
| `focused` | `true` escolhe a janela que o gerenciador de janelas informa como ativa; `false` escolhe todas as outras |
| `opacity` | Porcentagem de opacidade, de 0 a 100, usada no lugar da `opacity` global e multiplicada pela opacidade do aplicativo |
| `blur` | `false` mantém nítido o conteúdo atrás da janela; `true` o desfoca como por padrão, enquanto `blur_radius` for maior que zero |
| `fade_ms` | Duração da abertura e do fechamento em milissegundos, de 0 a 65535, usada no lugar do `fade_ms` global |
| `shadow` | `false` deixa a janela sem sombra; `true` dá a ela uma sombra, seja qual for seu tipo ou suas margens, enquanto `shadow_radius` for maior que zero |

Uma regra precisa de ao menos um dos quatro primeiros campos, que escolhem janelas, e de ao menos um dos quatro últimos, que ela define. O texto precisa ser idêntico, inclusive em maiúsculas e minúsculas, e a janela precisa corresponder a todos os campos pelos quais a regra escolhe. Cada opção vem da primeira regra correspondente que a define, então regras específicas vêm antes das amplas; assim, `true` em uma regra mantém o desfoque de janelas para as quais uma regra posterior o desliga. Rode `xprop` e clique em uma janela para ver seus `WM_CLASS`, `_NET_WM_WINDOW_TYPE` e `_NET_WM_NAME`.

O Compust lê essas propriedades da janela do aplicativo, dentro da moldura do gerenciador de janelas, e as lê de novo quando mudam, como quando um título muda. Uma janela que não declara nenhum tipo conhecido pelo Compust é `dialog` quando é transitória para outra janela e o gerenciador de janelas a controla, e `normal` nos demais casos, como determina a EWMH. Uma propriedade ausente ou malformada não corresponde a nenhum texto. Uma janela em fechamento mantém as regras que tinha enquanto o fade termina.

`focused` acompanha o `_NET_ACTIVE_WINDOW` da raiz, no qual o gerenciador de janelas indica a janela de aplicativo ativa; a moldura em volta dessa janela recebe o foco junto com ela. Uma regra com `focused = false` também corresponde a menus, dicas de ferramenta e docks, então uma regra feita para esmaecer janelas inativas costuma trazer também `window_type = "normal"`. Em um gerenciador de janelas que não define essa propriedade, toda janela conta como focada, e uma regra assim não muda nada. Openbox 3.6.1, i3 4.25.1, bspwm 0.9.12 e dwm 6.8 a definem; o Xmonad 0.18.1 só a define com `XMonad.Hooks.EwmhDesktops`. Como as sombras, `focused` está no `main` e entra na próxima beta.

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

O protótipo repinta apenas a área da tela que mudou, e cada janela desfocada guarda seu fundo desfocado até que algo abaixo dela mude, o que a desfoca de novo em toda a sua área de alcance. No [desktop AMD/XLibre registrado](docs/DESKTOP_TESTING.pt-BR.md#sessão-registrada-do-desfoque-em-pirâmide-2026-10-03), uma janela translúcida em tela cheia com desfoque manteve 60 quadros por segundo enquanto o Xorg usava cerca de 4% de um núcleo. Janelas escondidas atrás de janelas opacas não são pintadas. O renderizador de GPU opcional ([registrado em um desktop](docs/DESKTOP_TESTING.pt-BR.md#renderizador-de-gpu-registrado-2026-10-04)) desenha os mesmos quadros que o XRender com diferença de até dois níveis de cor, com CPU total parecida ali, maior quando janelas são redimensionadas, e cerca de 62 MiB a mais de memória para o driver GL. O [marco de renderização](docs/ROADMAP.pt-BR.md#trabalho-de-renderização-quatro-etapas-concluídas-em-um-desktop) mediu esses custos; seus [primeiros registros](docs/DESKTOP_TESTING.pt-BR.md#cenas-de-benchmark-registradas-2026-10-03) registram cargas do Compust em uma máquina antes das mudanças na repintura; os registros seguintes medem suas próprias mudanças de renderização. As sombras são pretas e retangulares, e janelas com formato não projetam nenhuma; [seus pixels, regras de foco e suspensão em tela cheia estão registrados em uma tela AMD/XLibre nos dois pintores](docs/DESKTOP_TESTING.pt-BR.md#sombras-foco-e-suspensão-em-tela-cheia-registrados-2026-10-04). A suspensão da composição em tela cheia exige uma janela sobre a raiz inteira, então nunca vale para um monitor entre vários. Esse registro usa janelas sintéticas; aplicativos reais em tela cheia ainda precisam de validação. Não há cantos arredondados, animações de movimento ou escala nem compatibilidade com arquivos do picom.

O marco planejado de **Animações de janelas** no [roteiro](docs/ROADMAP.pt-BR.md) amplia o fade existente com pop, slide e curvas, escolhidos por janela, e o marco planejado de [Cantos arredondados](docs/ROADMAP.pt-BR.md#cantos-arredondados-planejados) arredonda as janelas, o desfoque atrás delas e suas sombras, com um raio global ou definido por regras. O binário atual não aceita os exemplos de configuração de Animações de janelas; o `main` aceita `corner_radius`, mas ainda não desenha cantos arredondados.

Um processo atende uma tela X; uma raiz com vários monitores é composta como uma única superfície. Agendamento independente por monitor, HDR/gerenciamento de cores, VRR, importação DMA-BUF, sincronização explícita e extensões exclusivas do XLibre não estão implementados ou certificados. O hotplug físico foi verificado apenas no [desktop AMD/XLibre registrado](docs/DESKTOP_TESTING.pt-BR.md#sessão-registrada-em-hardware-2026-10-03) e no [laptop Intel/Xorg registrado](docs/DESKTOP_TESTING.pt-BR.md#sessão-registrada-de-hotplug-em-intelxorg-2026-10-03). Wayland nativo está fora do escopo atual.

## Contexto e licença

O projeto se inspira no modelo de compositor independente do [picom](https://github.com/yshui/picom), cuja origem inclui o Compton. O picom já oferece animações e efeitos; a contribuição pretendida pelo Compust é uma implementação acessível em Rust, com melhorias medidas ao longo do desenvolvimento. Este repositório contém uma implementação nova, não uma tradução do código do picom.

Referências: [x11rb](https://docs.rs/x11rb/0.13.2/x11rb/), [compositores na especificação EWMH](https://specifications.freedesktop.org/wm/latest/ar01s08.html) e [XLibre](https://github.com/X11Libre/xserver). Licença [MIT](LICENSE).
