# Validação de desktops

[English (US)](DESKTOP_TESTING.md) | [Português (Brasil)](DESKTOP_TESTING.pt-BR.md)

Este guia cobre o trabalho reproduzível de desktops da etapa 3 da beta. O Xmonad dentro do Xephyr exercita um gerenciador de janelas e um servidor X reais. O Xephyr hospedado pelo Xvfb usa renderização por software; esses resultados não validam driver de GPU, monitor físico nem apresentação sem tearing. Sessões AMD/XLibre registradas abaixo cobrem as transições físicas de monitores da etapa 2 e os cenários de desktop do probe em hardware da etapa 3. Um laptop Intel/Xorg registrado cobre os mesmos cenários e uma mudança de modo em seu único painel. Outros hardwares, gerenciadores de janelas e aplicativos reais continuam em aberto.

## Executar a medição isolada

Instale a toolchain Rust fixada pelo projeto, um linker C, GHC com as bibliotecas `xmonad` e `xmonad-contrib`, Xvfb, Xephyr, `xprop`, `xdpyinfo`, `xrandr` e utilitários Linux como `timeout`, `getconf` e `sha256sum`. Execute na raiz do repositório. O script aloca os dois displays automaticamente e inicia uma configuração privada do Xmonad; pode rodar em uma sessão Wayland ou sem desktop. Defina `WINDOW_MANAGER` como `openbox`, `i3` ou `bspwm` para testar um deles, cada um com sua própria configuração privada ([Openbox](../tools/desktop/openbox.xml), [i3](../tools/desktop/i3.config), [bspwm](../tools/desktop/bspwmrc)); isso exige o gerenciador escolhido instalado, e não GHC nem Xmonad.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
cargo build --release --locked --bin compust --example desktop_probe
mkdir -p artifacts
tools/desktop-check.sh artifacts/desktop-present present
tools/desktop-check.sh artifacts/desktop-direct direct
tools/desktop-check.sh artifacts/desktop-effects effects
```

Cada diretório de relatório precisa ser novo. Omitir o modo seleciona `present`; `--help` descreve o comando. Defina `XVFB` e `XEPHYR` com caminhos de executáveis alternativos para testar outro build de servidor, incluindo o Xephyr do XLibre. Eles não selecionam o display do seu desktop atual. `SECONDS_PER_PHASE` aceita 1–30 segundos e usa 10 por padrão. Execute as medições em sequência, sem outros builds ou benchmarks em andamento. Defina `DESKTOP_DISPLAY` e `SERVER_PID` para usar um servidor X dedicado já existente em vez de iniciar Xvfb e Xephyr; o script nunca encerra esse servidor. O [procedimento em hardware](#executar-as-verificações-de-desktop-em-hardware) usa esse modo.

As configurações `present` e `direct` usam `opacity = 100`, `fade_ms = 0`, `blur_radius = 0` e `max_fps = 120`. A [configuração Present](../tools/desktop/compust.toml) usa `vsync = true`; a [configuração direta](../tools/desktop/compust-direct.toml) usa `vsync = false`. Esses dois modos, portanto, não incluem os custos de fade, translucidez e desfoque. A [configuração de efeitos](../tools/desktop/compust-effects.toml) usa Present com os fades padrão de 180 ms e raio de desfoque 4. No modo `effects`, o probe também deixa a janela sobrevivente 50% translúcida antes de medir, então cada atualização mistura essa janela e desfoca a tela inteira atrás dela.

O probe verifica organização das janelas pelo gerenciador, remoção de popup override-redirect, tela cheia via EWMH e restauração, troca para um workspace vazio, definição e remoção de um papel de parede na raiz nesse workspace, retorno, 32 sequências rápidas de criação/map/destruição e atualização de uma janela sobrevivente. Cada cena verifica pixels reais do overlay. Com o Openbox, as janelas mantêm seu tamanho dentro de molduras decoradas; por isso o probe também exige uma moldura com reparenting e barra de título pintada, sobrepõe as duas janelas e traz cada uma para a frente, e minimiza e restaura uma delas. Com o i3, um gerenciador tiling que também faz reparenting, o probe exige as mesmas barras de título nas molduras e mapeia antes uma pequena janela-âncora que o i3 envia para um segundo workspace, porque o i3 só mantém workspaces que têm foco ou alguma janela. Com o bspwm, um gerenciador tiling que não faz reparenting, o script espera a configuração do bspwm criar o segundo desktop. O bspwm 0.9.12 às vezes mantém o espaço de uma janela destruída logo depois do seu pedido de mapeamento; por isso, nele, a última janela não precisa crescer até ocupar a tela, desde que o espaço deixado mostre o fundo. O script também exige encerramento bem-sucedido do compositor após SIGTERM. A inicialização dos displays espera por `-displayfd`; seleção do compositor, gerenciador e cenas usam notificações X11 com prazo máximo.

Após dois segundos de aquecimento, o probe mede uma fase ociosa e outra com uma janela grande, opaca exceto no modo `effects`, alternando vermelho e azul a uma frequência solicitada de 60 atualizações por segundo. Ele registra separadamente CPU de Compust, servidor X, Xvfb hospedeiro quando houver, Xmonad e probe, com RSS no início e no fim de cada fase. Os percentuais consideram um núcleo como 100%; zero significa que nenhum tick de CPU foi observado naquele intervalo. Valores de RSS nos extremos não constituem um teste prolongado de vazamentos.

Nos modos `present` e `effects`, a fase ativa precisa receber várias conclusões Present do compositor. `frames.csv` contém timestamps UST do servidor, valores MSC, seriais e modos de conclusão. Calcule intervalos somente entre registros sucessivos da mesma fase. Em servidores aninhados, são tempos de conclusão por software; em hardware, acompanham o vblank do CRTC. Nenhum deles é latência entre entrada e exibição.

No modo `direct`, o probe exige notificações Damage do overlay durante a atividade e rejeita qualquer conclusão Present do compositor. `frames.csv` contém apenas o cabeçalho: não há medição de regularidade via Present para cópia direta. Contagens Damage comprovam atividade de renderização, não a taxa de quadros exibidos. Este probe ainda exige a extensão Present do servidor para detectar um caminho selecionado incorretamente; o Compust de produção pode executar sem essa extensão.

## Executar as verificações de desktop em hardware

[`tools/hardware-session.sh`](../tools/hardware-session.sh) executa os três modos com GPU e monitor físico. Ele é o cliente de um novo servidor X iniciado em um console de texto, então não substitui nem perturba uma sessão de trabalho. Por exemplo, pressione Ctrl+Alt+F3, faça login e execute:

```sh
cd caminho/para/compust
env SESSION_OUTPUT=HDMI-1 startx "$PWD/tools/hardware-session.sh" artifacts/hardware-desktop -- :20
```

Escolha um número de display livre e o nome de uma saída exibida pelo `xrandr`; sem `SESSION_OUTPUT`, é usada a primeira saída conectada. O script para se a saída indicada não estiver conectada: o `xrandr` ignora um nome que não conhece, e todas as saídas seriam desligadas. O script deixa somente essa saída ativa no modo preferido, desativa o apagamento da tela e mantém um cliente conectado para que o servidor não seja reiniciado entre as execuções. Ele registra saídas, provedores e GPU e depois executa `present`, `direct` e `effects` no novo servidor, continuando após um modo com falha. `WINDOW_MANAGER` também seleciona o gerenciador de janelas aqui. Por fim, copia o log do servidor quando ele pode ser lido e encerra, terminando a sessão. Não use teclado nem mouse até aparecer `Hardware session finished`; depois faça logout e volte à sua sessão habitual. Revise `xorg.log` antes de compartilhá-lo: ele inclui números de série dos monitores e a linha de comando do kernel.

## Verificar sombras, foco e suspensão em tela cheia

[`tools/features-check.sh`](../tools/features-check.sh) executa `desktop_probe --features` em um display isolado sem gerenciador de janelas. A [configuração](../tools/desktop/compust-features.toml) desativa fades e desfoque, liga sombras de raio 12 com opacidade máxima e deslocamentos de 4 e 6 pixels, liga a suspensão em tela cheia e deixa as janelas inativas do teste de foco a 50% de opacidade. Compile os dois binários de release juntos e ensaie no Xvfb:

```sh
cargo build --release --locked --bin compust --example desktop_probe
mkdir -p artifacts
SECONDS_PER_PHASE=1 tools/features-check.sh artifacts/features-xvfb xrender
```

Cada diretório de relatório precisa ser novo. Esse ensaio inicia o Xvfb automaticamente e testa somente o XRender: o Xvfb não tem DRI3 e não pode validar o pintor de GPU. A seleção padrão, `both`, executa XRender seguido de GL no mesmo servidor dedicado e compara pixel a pixel as imagens de sombras e foco. Ela falha se o GL não abriu o pintor de GPU ou voltou ao XRender. `DESKTOP_DISPLAY` e `SERVER_PID` selecionam um servidor dedicado existente, que o runner nunca encerra; `XVFB` seleciona outro executável do Xvfb.

Para testar no hardware, faça login em um console de texto, por exemplo Ctrl+Alt+F3, e execute:

```sh
cd ~/compust
env SESSION_OUTPUT=HDMI-1 startx "$PWD/tools/hardware-session.sh" artifacts/features-hardware --features -- :20
```

Escolha uma saída conectada e um número de display livre, como no procedimento de desktop acima. Omitir `SESSION_OUTPUT` seleciona a primeira saída conectada. Nenhum gerenciador de janelas específico é necessário. Deixe essa sessão em primeiro plano até aparecer `Hardware session finished`; ela volta ao console, e você pode retornar ao desktop habitual.

O probe confere cada pixel da cena de sombras contra uma referência independente dos filtros de caixa, incluindo o interior intacto da janela, as quatro bordas da sombra, os cantos e o fundo branco. Os testes e as comparações entre pintores admitem dois níveis por canal RGB por arredondamento. Escrever `_NET_ACTIVE_WINDOW` testa as duas direções do foco, `NONE` e o comportamento após remover a propriedade. As cenas esperam notificações X11 com prazo máximo.

A mesma janela opaca de tela cheia alterna vermelho e azul a uma frequência solicitada de 60 atualizações por segundo nas fases `composed` e `suspended`. Um diálogo pequeno por cima mantém a composição ativa; removê-lo precisa desmapear o overlay, interromper conclusões Present e Damage do overlay e deixar o desenho chegar diretamente à raiz. Cada estado tem dois segundos de aquecimento e `SECONDS_PER_PHASE` segundos de medição (1–30, padrão 10). Abrir outro diálogo precisa recapturar o conteúdo desenhado durante a suspensão; remover a janela de tela cheia precisa restaurar o fundo. A CPU é registrada separadamente para Compust, servidor e probe, sem exigir uma economia mínima em execuções curtas ou por software.

O relatório de cada pintor inclui `shadows.csv`, imagens PPM de sombras e foco, `fullscreen-resumed.ppm`, `fullscreen.csv`, `processes.csv`, `frames.csv`, topologia e recursos XRes, configuração exata, logs e evidência de encerramento via SIGTERM. O GL grava um CSV de comparação para cada imagem de sombras e foco. A raiz reúne patches e hashes dos fontes, hashes dos binários, commit base e alterações locais, dados de CPU/servidor e horários. Um ensaio ou uma volta ao XRender não constitui validação em hardware. A [sessão registrada na RX 9060 XT](#sombras-foco-e-suspensão-em-tela-cheia-registrados-2026-10-04) cobre essas cenas nos dois pintores em uma tela AMD/XLibre.

## Preservar a identificação das evidências

O relatório inclui extensões do servidor, geometria das saídas, modelo de CPU, kernel, versões de Rust e do gerenciador, configuração exata, medições dos processos, capturas das cenas, logs e hashes dos executáveis. `commit.txt` identifica a revisão base; `worktree.txt`, `source.patch` e `source-sha256.txt` registram alterações locais. Uma execução com mudanças deve ser descrita como aquela revisão mais essas mudanças, sem atribuir o resultado ao commit limpo.

Preserve os CSVs brutos junto aos resumos. Mantenha somente capturas das janelas isoladas de teste nos artefatos locais; revise os logs antes de compartilhá-los. Os [registros anteriores](benchmarks/2026-10-03/) contêm as primeiras medições Present em `1409a919dd0c91ccaad3fbb323df4b33628554fd`. Os metadados arquivados nelas são menos completos que o relatório gerado pelo script atual.

## Medição registrada: 2026-10-03

Quatro execuções sequenciais passaram no CachyOS, Linux 7.2.8-arch1-1, AMD Ryzen 5 5600GT, Rust 1.95.0 e Xmonad 0.18.1. Cada uma usou uma tela Xephyr de 1280×800×24 hospedada pelo Xorg Xvfb 21.1.24. Os servidores aninhados foram Xorg 21.1.24 e XLibre 25.1.9. O RandR informou taxa virtual de 0,00 Hz; a carga solicitou 60 atualizações por segundo.

A base testada foi `cb0796c8e42a95d3c80ab11c557b75809f791d43` mais as [alterações registradas do probe/script](benchmarks/2026-10-03/presentation/source.patch) e a configuração copiada do modo direto. Os [hashes dos fontes](benchmarks/2026-10-03/presentation/source-sha256.txt) coincidiram nas quatro execuções. O SHA-256 do binário de produção do compositor foi `5452d6f590502b1437b8064521d8d82dddd1095d959078fe61ae2d340969a005`, idêntico ao binário registrado anteriormente. O patch dos fontes não inclui o novo arquivo de configuração; cada execução arquiva seu `compust.toml` real separadamente.

Cada fase ativa durou aproximadamente 10 segundos e emitiu 600 atualizações. Os valores de CPU abaixo são percentuais de um núcleo; o RSS está em KiB. Os links levam às medições dos processos, com as demais evidências no mesmo diretório.

| Servidor aninhado / caminho | CPU ativa Compust | CPU ativa Xephyr | RSS Compust antes → depois | Conclusões Present / notificações Damage |
| --- | ---: | ---: | ---: | ---: |
| [Xorg / Present](benchmarks/2026-10-03/presentation/xorg-present/processes.csv) | 0,4% | 10,5% | 3.820 → 3.820 | 599 / 599 |
| [Xorg / direto](benchmarks/2026-10-03/presentation/xorg-direct/processes.csv) | 0,3% | 13,3% | 3.876 → 3.876 | 0 / 600 |
| [XLibre / Present](benchmarks/2026-10-03/presentation/xlibre-present/processes.csv) | 0,4% | 13,7% | 3.888 → 3.888 | 599 / 599 |
| [XLibre / direto](benchmarks/2026-10-03/presentation/xlibre-direct/processes.csv) | 0,2% | 9,4% | 3.880 → 3.880 | 0 / 600 |

Todos os processos medidos registraram zero ticks de CPU ociosa e RSS inalterado nos extremos das duas fases. Cada execução Present observou uma conclusão durante a ociosidade após o aquecimento; cada execução direta não observou notificações Damage ociosas. O Xvfb hospedeiro usou 2,9–4,5% de CPU ativa e o probe 0,3–0,4%, registrados separadamente. Este probe observa Damage nos dois modos, então as medições incluem o custo dessa observação. São referências de execuções individuais, não evidência de superioridade de um servidor ou modo.

| Intervalos Present, fase ativa | Mediana | p95 | Máximo |
| --- | ---: | ---: | ---: |
| [Xorg, 598 intervalos](benchmarks/2026-10-03/presentation/xorg-present/frames.csv) | 16,669 ms | 17,740 ms | 18,728 ms |
| [XLibre, 598 intervalos](benchmarks/2026-10-03/presentation/xlibre-present/frames.csv) | 16,669 ms | 17,807 ms | 18,890 ms |

Os intervalos são diferenças UST sucessivas divididas por 1.000; mediana e p95 usam a convenção nearest-rank. Todos os modos de conclusão registrados foram `COPY`. Os arquivos de quadros do modo direto não contêm amostras de tempo.

Dois testes negativos trocaram somente as configurações em cópias temporárias do script. Exigir Present de um compositor direto [falhou por ausência de conclusões](benchmarks/2026-10-03/presentation/rejected-present/probe.log); exigir cópia direta de um compositor Present [falhou por conclusões inesperadas](benchmarks/2026-10-03/presentation/rejected-direct/probe.log). Ambos encerraram com status 1; as quatro execuções compatíveis encerraram com sucesso após verificar pixels e desligamento via SIGTERM.

Os logs do Xmonad mantêm diagnósticos `BadAtom` e `getWindowAttributes` também presentes nos arquivos anteriores. A causa não foi resolvida neste incremento; todas as verificações de cenas passaram. O aviso de consulta de gama do Xephyr e sua taxa virtual não validam o comportamento de monitores físicos. Hotplug em hardware, drivers de GPU, gerenciadores com decoração/reparenting e desempenho com efeitos pesados permanecem sem verificação aqui.

## Amostrar transições de monitores

`tools/hotplug-check.sh` executa o Compust em uma sessão X11 existente enquanto um operador altera os monitores; ele não inicia servidor nem gerenciador de janelas. Use uma sessão Xorg ou XLibre dedicada. Pare antes o compositor dela, guarde o comando que o restaura e mantenha um terminal em um monitor que continuará conectado. `SERVER_PID` identifica o servidor X para medir CPU e RSS; `WM_PID` é opcional. Ajuste os nomes de processos abaixo ao seu servidor e gerenciador.

```sh
cargo build --release --locked --bin compust --example desktop_probe
SERVER_PID=$(pgrep -x Xorg) WM_PID=$(pgrep -n xmonad) \
    tools/hotplug-check.sh artifacts/hotplug-present present
```

Depois de `READY`, faça uma transição por vez, espere o desktop se estabilizar e digite `sample RÓTULO`; os rótulos usam letras minúsculas sem acento, dígitos e hífens. Digite `quit` ao terminar. O script então encerra o Compust com SIGTERM e exige saída bem-sucedida. Cada amostra faz o seguinte:

- grava em `topology.txt` o tamanho da raiz, a conexão, o CRTC, a geometria, o modo e a taxa de atualização de cada saída, a saída principal e os monitores RandR ativos;
- move um marcador override-redirect de 128×96 para perto de cantos opostos de cada monitor ativo e sobre cada borda compartilhada por dois monitores, alternando verde e vermelho até o overlay exibir cada cor;
- repinta uma janela gerenciada no lugar, sem movê-la nem redimensioná-la, e exige cada cor no overlay; um compositor que ainda mostra o pixmap da janela de antes de um redimensionamento falha aqui;
- mede dois segundos de aquecimento e fases ociosa e ativa de dez segundos, como na medição isolada, gravando `processes.csv` e `frames.csv`;
- grava em `resources.csv` as contagens de recursos XRes do compositor e o total de bytes dos pixmaps dele.

O probe mapeia uma janela gerenciada, que um gerenciador de janelas tiling acrescenta ao layout atual, e não troca workspaces. Mantenha essa janela visível e descoberta. Aplicativos comuns podem continuar abertos; as atualizações deles entram na medição ociosa. As verificações de pixels leem o framebuffer do servidor X pelo overlay, não a luz emitida pelos painéis. O relatório omite os dados EDID de `xrandr --verbose` porque eles contêm números de série dos monitores. Os registros locais de processos ainda contêm linhas de comando, então revise o relatório antes de compartilhá-lo.

Uma sequência útil começa com uma referência inicial e depois muda o modo, a disposição e desativa uma saída, restaurando após cada mudança. Para cada conector, registre amostras com ele desconectado enquanto o CRTC ainda está atribuído, após a reação do desktop (`xrandr --auto` abaixo), reconectado e restaurado. Execute a sequência uma vez por modo de apresentação.

## Executar as cenas de benchmark

`tools/bench.sh` mede as próprias cenas fixas do Compust em uma sessão X11 existente; ele não inicia servidor nem gerenciador de janelas. Pare antes o compositor da sessão e guarde o comando que o restaura. As [configurações](../tools/bench/) desativam fades, usam opacidade total e `vsync = true` e escolhem XRender ou o pintor GL opcional, com raio de 4 pixels nas variantes com desfoque. `COMPOSITORS` aceita `compust` (o padrão), `compust-gl` ou os dois separados por espaço. `SCENES` escolhe um subconjunto. Nenhum outro compositor é executado nem exigido.

```sh
cargo build --release --locked --bin compust --example desktop_probe
SERVER_PID=$(pgrep -x Xorg) WM_PID=$(pgrep -x dwm) tools/bench.sh artifacts/bench
```

Cada cena mapeia um fundo sobre toda a raiz e depois janelas override-redirect, para que nenhum gerenciador as posicione e a geometria seja igual em qualquer desktop. Depois de dois segundos de aquecimento, a sonda mede por `SECONDS_PER_PHASE`, vinte segundos por padrão, como em todas as execuções registradas. O tempo de CPU é contado em ticks de escalonamento de 10 ms, então uma fase de 20 segundos distingue 0,05 ponto de um núcleo.

| Cena | Carga |
| --- | --- |
| `idle` | Somente o fundo |
| `small-update` | Uma janela de 64×64 alternando vermelho e azul 60 vezes por segundo |
| `fullscreen-translucent` | Uma janela a 50% cobrindo a raiz, alternando do mesmo modo |
| `eight-translucent` | Oito janelas de 480×360 sobrepostas a 50%; a de cima alterna |
| `covered` | As mesmas oito janelas sob uma janela opaca que cobre a raiz e alterna |
| `move-resize` | Uma janela movida e redimensionada 60 vezes por segundo |
| `open-close` | Janelas, uma por vez, durante toda a fase, mapeadas até o overlay mostrá-las e destruídas até o overlay mostrar o fundo; os registros até o de oclusão de 2026-10-04, inclusive, rodaram 100 ciclos, cerca de 3,3 segundos |

As cenas translúcidas e a coberta rodam sem desfoque e, com o sufixo `:blur`, com ele. Todas cabem em uma tela de 1366×768; em uma maior, só a janela de tela cheia cresce.

Cada diretório de cena contém `summary.csv`, com intervalos do Present, CPU, RSS, latências de abertura e fechamento e carga da GPU; `frames.csv`, com cada conclusão do Present; `processes.csv`; `latency.csv` em `open-close`; `topology.txt`; `resources.csv`; e os logs do compositor e da sonda. A raiz do relatório reúne as linhas de resumo no próprio `summary.csv` e registra o commit, as configurações do Compust e os hashes dos binários. A CPU é uma fração de um núcleo e, como nas outras medições, exclui o tempo de GPU. No amdgpu, a sonda também amostra `gpu_busy_percent` dez vezes por segundo; `GPU_BUSY` indica outro arquivo de carga. Uma latência de abertura ou fechamento vai da requisição até a sonda ler a mudança no overlay após uma conclusão do Present ou um evento Damage, então inclui uma ida e volta de `GetImage`. `skipped_vblanks` conta vblanks sem conclusão entre quadros consecutivos e só faz sentido em cenas que atualizam a cada vblank. Em uma raiz com vários monitores, o Present pode trocar o CRTC que acompanha, e o MSC do novo CRTC tem outra base; `msc_discontinuities` conta saltos de MSC incompatíveis com o tempo entre os quadros, cujos vblanks perdidos são estimados a partir desse tempo. O XRes conta pixmaps do X, mas não buffers GL, então as contagens do XRes sozinhas não descrevem a memória total do pintor GL.

O Present não conclui quadros enquanto o DPMS mantém os monitores desligados. O script os liga, desativa a proteção de tela e o DPMS durante a execução e restaura as opções anteriores ao final. Execute-o em uma sessão dedicada ou em um desktop ocioso: os redesenhos de outros aplicativos entram em todas as cenas, com qualquer pintor do Compust.

## Sessão registrada em hardware: 2026-10-03

A sessão rodou no CachyOS com Linux 7.2.8-2-cachyos e XLibre 25.1.9 nativo usando o driver modesetting. Ela usou o driver de kernel amdgpu, um AMD Ryzen 5 5600GT com gráficos Radeon Vega integrados, Mesa 26.2.4, libdrm 2.4.134 e Xmonad 0.18.1 com xmonad-contrib 0.18.2. O desktop manteve sua configuração habitual do Xmonad, barra de status e bandeja. HDMI-1 era a saída principal, à direita; DP-1, um adaptador DisplayPort para VGA, ficava à esquerda. Ambas usaram 1920×1080 a 60 Hz, formando uma raiz de 3840×1080. O Picom foi parado antes das execuções e reiniciado depois. As [versões dos pacotes](benchmarks/2026-10-03/hardware/packages.txt) estão arquivadas com os relatórios.

A base testada foi `36bd7e8876a6abea564907a610bcabf62794db77` mais as [alterações registradas](benchmarks/2026-10-03/hardware/present/source.patch), incluindo a correção abaixo. Os [hashes dos fontes](benchmarks/2026-10-03/hardware/present/source-sha256.txt) coincidem nas duas execuções, e o SHA-256 do binário do compositor foi `1220413a892149d2c12bbf56dded047c20bf7956e5063dc246c61a5989ddb2bf`. Esse hash vem do comando de build combinado acima; compilar apenas o compositor gera outro binário, porque o probe ativa um recurso adicional do x11rb. As duas execuções usaram as configurações [Present](../tools/desktop/compust.toml) e [direta](../tools/desktop/compust-direct.toml) da medição isolada, com fades e desfoque desativados.

### Travamento encontrado e corrigido

A primeira execução Present [parou na segunda amostra](benchmarks/2026-10-03/hardware/stall/probe.log): após `xrandr --output HDMI-1 --mode 1280x720`, o marcador não foi atualizado em cinco segundos. Um build com [registros adicionais](benchmarks/2026-10-03/hardware/stall/diagnosis/instrumentation.patch) reproduziu o problema. Os eventos RandR mostram uma mudança de CRTC ainda no tamanho 3840×1080, o redimensionamento da raiz para 3200×1080 e uma segunda mudança de CRTC, tudo enquanto o serial 324 estava pendente. O servidor nunca enviou os eventos de conclusão e ociosidade desse serial, e o Compust [continuou esperando](benchmarks/2026-10-03/hardware/stall/diagnosis/compust-debug.log) em vez de recriar seus buffers. O [roteiro](ROADMAP.pt-BR.md#etapa-2-em-hardware-hotplug-físico-no-xlibre-com-amd) descreve a correção e as regressões.

### Resultados

As duas execuções passaram nas quinze amostras e encerraram com sucesso após SIGTERM. As linhas correspondem aos diretórios `001-baseline` a `015-dp-restored` nos relatórios [Present](benchmarks/2026-10-03/hardware/present/) e [direto](benchmarks/2026-10-03/hardware/direct/). A CPU é o percentual de um núcleo na fase ativa; os bytes são idênticos nos dois modos.

| Amostra | Raiz | Monitores ativos | Present: CPU Compust / Xorg | Direto: CPU Compust / Xorg | Bytes de pixmaps |
| --- | --- | ---: | ---: | ---: | ---: |
| Referência inicial | 3840×1080 | 2 | 0,5% / 4,1% | 0,3% / 3,7% | 49.580.389 |
| HDMI-1 em 1280×720 | 3200×1080 | 2 | 0,4% / 4,2% | 0,4% / 3,8% | 39.544.229 |
| Modo restaurado | 3840×1080 | 2 | 0,4% / 4,0% | 0,4% / 3,8% | 49.580.389 |
| Disposição vertical | 1920×2160 | 2 | 0,4% / 3,9% | 0,3% / 3,8% | 49.763.749 |
| Disposição restaurada | 3840×1080 | 2 | 0,5% / 3,9% | 0,3% / 3,9% | 49.580.389 |
| DP-1 desligado | 1920×1080 | 1 | 0,4% / 3,4% | 0,3% / 3,3% | 25.000.149 |
| DP-1 religado | 3840×1080 | 2 | 0,5% / 4,0% | 0,4% / 3,6% | 49.580.389 |
| HDMI-1 desconectado | 3840×1080 | 2 | 0,4% / 4,1% | 0,4% / 3,9% | 49.580.389 |
| `xrandr --auto` | 1920×1080 | 1 | 0,4% / 3,4% | 0,4% / 3,4% | 25.000.149 |
| HDMI-1 reconectado | 1920×1080 | 1 | 0,4% / 3,4% | 0,3% / 3,7% | 25.000.149 |
| Disposição restaurada | 3840×1080 | 2 | 0,4% / 4,1% | 0,4% / 4,0% | 49.580.389 |
| DP-1 desconectado | 3840×1080 | 2 | 0,4% / 4,2% | 0,5% / 4,4% | 49.580.389 |
| `xrandr --auto` | 1920×1080 | 1 | 0,4% / 3,3% | 0,4% / 3,2% | 25.000.149 |
| DP-1 reconectado | 1920×1080 | 1 | 0,3% / 3,5% | 0,4% / 3,6% | 25.000.149 |
| Disposição restaurada | 3840×1080 | 2 | 0,3% / 4,1% | 0,4% / 3,8% | 49.580.389 |

Cada fase ativa emitiu 600 atualizações do marcador. As amostras Present receberam 600 ou 601 conclusões, todas `COPY`. As amostras diretas não receberam conclusões e tiveram 709–728 notificações Damage do overlay, incluindo atualizações causadas por outros clientes. Nas fases ociosas, o Compust usou 0,0–0,2% e o Xorg 0,9–1,5% de um núcleo; os próprios clientes do desktop ainda causaram 126–147 atualizações do compositor a cada dez segundos.

Em 8.988 intervalos Present das fases ativas, a mediana e o p95 nearest-rank foram 16,667 ms e o máximo foi 16,670 ms. As amostras em que o DP-1 cobria a maior parte da raiz, com o HDMI-1 em 1280×720 ou desligado, tiveram mediana de 16,635 ms, o que indica que o Present acompanhou o CRTC do DP-1; as demais tiveram 16,667 ms. Com o HDMI-1 desconectado, mas ainda com um CRTC atribuído, os intervalos continuaram em 16,667 ms.

Cada topologia repetida reproduziu o mesmo `resources.csv` em cada modo. Com um único monitor ativo, o compositor manteve um pixmap, uma imagem e um objeto Damage de janela a menos, porque havia uma janela mapeada a menos; o modo direto não tem seleção de eventos Present. O RSS do Compust passou de 3.884 para 3.896 KiB na execução Present, sem mudanças a partir da terceira amostra, e de 3.760 para 3.772 KiB na direta, sem mudanças a partir da sexta. O RSS do Xorg subiu de 118.604 para 119.244 KiB na primeira mudança de modo e chegou a 119.340 KiB ao fim da execução direta.

Esses resultados validam transições de monitores somente neste ambiente. Eles não cobrem outros drivers de GPU, Xorg em hardware, taxas de atualização mistas, mais de dois monitores, fades, desfoque ou sessões longas. Desligar o DP-1 pelo `xrandr` também moveu o HDMI-1 para a origem e reduziu a raiz. Mudanças de CRTC sem redimensionar a raiz são cobertas pela regressão no Xvfb. O hotplug do DP-1 é a conexão DisplayPort do adaptador. As verificações do marcador comprovam atualizações no servidor, não o que cada painel exibiu.

## Medição registrada com efeitos: 2026-10-03

Seis execuções sequenciais repetiram a medição isolada nos três modos com Xorg Xephyr 21.1.24 e XLibre Xephyr 25.1.9. A base foi `0ef4d14f0e5e81fff8cf3689f008961e3c1ed15e` mais as [alterações registradas](benchmarks/2026-10-03/effects/xorg-present/source.patch), com o binário do compositor `1220413a892149d2c12bbf56dded047c20bf7956e5063dc246c61a5989ddb2bf`. Todas as execuções passaram em todos os cenários, incluindo a troca de papel de parede.

| Servidor aninhado / modo | CPU ativa Compust | CPU ativa Xephyr | Quadros em 10 s | Intervalo mediana / p95 |
| --- | ---: | ---: | ---: | ---: |
| [Xorg / Present](benchmarks/2026-10-03/effects/xorg-present/processes.csv) | 0,4% | 10,4% | 599 | 16,679 / 17,676 ms |
| [Xorg / direto](benchmarks/2026-10-03/effects/xorg-direct/processes.csv) | 0,3% | 10,4% | 600 Damage | — |
| [Xorg / efeitos](benchmarks/2026-10-03/effects/xorg-effects/processes.csv) | 0,0% | 85,8% | 85 | 116,681 / 118,518 ms |
| [XLibre / Present](benchmarks/2026-10-03/effects/xlibre-present/processes.csv) | 0,6% | 14,3% | 598 | 16,673 / 17,711 ms |
| [XLibre / direto](benchmarks/2026-10-03/effects/xlibre-direct/processes.csv) | 0,3% | 9,9% | 600 Damage | — |
| [XLibre / efeitos](benchmarks/2026-10-03/effects/xlibre-effects/processes.csv) | 0,0% | 86,1% | 85 | 116,640 / 118,027 ms |

Os resultados Present e diretos coincidem com a medição anterior dentro da variação entre execuções. Nas execuções diretas, os quadros são notificações Damage do overlay. Com a janela sobrevivente translúcida e raio de desfoque 4, o Xephyr gastou quase um núcleo inteiro em quadros de 1280×800 e entregou cerca de 8,5 por segundo, enquanto o Compust não registrou ticks de CPU.

## Sessão de desktop registrada em hardware: 2026-10-03

A [máquina AMD/XLibre acima](#sessão-registrada-em-hardware-2026-10-03) executou um servidor XLibre 25.1.9 dedicado no vt3, iniciado pelo `startx` em um console de texto enquanto a sessão habitual continuava no vt2. O [log do servidor](benchmarks/2026-10-03/desktop-hardware/xorg.log) informa glamor sobre radeonsi com OpenGL 4.6 e TearFree ativado pelo padrão do driver modesetting. HDMI-1 era a única saída ativa, em 1920×1080 a 60 Hz; o DP-1 ficou desligado. A árvore e os binários testados coincidem com os da medição aninhada com efeitos. Os três modos passaram em todos os cenários; o [log da sessão](benchmarks/2026-10-03/desktop-hardware/session.log) lista os resultados.

| Modo | CPU ativa Compust | CPU ativa Xorg | Quadros em 10 s | Intervalo mediana / p95 / máximo |
| --- | ---: | ---: | ---: | ---: |
| [Present](benchmarks/2026-10-03/desktop-hardware/present/processes.csv) | 0,3% | 3,1% | 600 | 16,667 / 16,667 / 16,667 ms |
| [Direto](benchmarks/2026-10-03/desktop-hardware/direct/processes.csv) | 0,3% | 3,5% | 600 Damage | — |
| [Efeitos](benchmarks/2026-10-03/desktop-hardware/effects/processes.csv) | 0,0% | 93,7% | 49 | 199,998 / 216,665 / 216,665 ms |

O Present acompanhou o vblank com exatidão: cada MSC da fase ativa avançou uma unidade. Nas fases ociosas, Compust e Xorg não registraram ticks de CPU nos modos Present e direto; no modo de efeitos, o Xorg usou 1,9% de um núcleo para concluir o último quadro do aquecimento. O RSS do Compust ficou entre 3.824 e 3.888 KiB e o do Xorg entre 92.820 e 93.152 KiB, sem mudanças dentro de cada fase.

Com desfoque, cada quadro levou 12 ou 13 vblanks enquanto o Xorg usava quase um núcleo inteiro, o que é mais lento por quadro que o Xephyr por software em 1280×800. O glamor acelera apenas filtragem nearest e bilinear; [`glamor_composite`](https://github.com/X11Libre/xserver/blob/b4b92c2374ec81ea979d53ad79fd0d0784bbf291/glamor/glamor_render.c#L1766-L1769) envia qualquer filtro de convolução para o caminho por software, que transfere pixmaps entre a memória da GPU e a da CPU. As duas passadas de convolução do Compust, portanto, rodam na CPU a cada atualização de uma janela translúcida. Nesta máquina, uma janela translúcida em tela cheia com raio de desfoque 4 limita a imagem a cerca de cinco quadros por segundo.

Essas execuções usam as janelas sintéticas do probe, um monitor, Xmonad e fases de dez segundos. Elas não cobrem aplicativos reais, gerenciadores com decoração ou reparenting, encerramento do servidor com o compositor em execução, taxas de atualização mistas nem outras GPUs. A [sessão do desfoque em pirâmide](#sessão-registrada-do-desfoque-em-pirâmide-2026-10-03) repete essas execuções depois da remoção da convolução.

## Sessão registrada do desfoque em pirâmide: 2026-10-03

Em seguida, as passadas de convolução foram substituídas por uma pirâmide bilinear, descrita no [roteiro](ROADMAP.pt-BR.md#desfoque-no-caminho-da-gpu). As mesmas execuções foram repetidas com o binário do compositor `6826205798e183b039d558a54794e506732bdc88560d8f6ae6a13eaa05baa35f`: seis [execuções aninhadas](benchmarks/2026-10-03/pyramid/) e uma [sessão dedicada em hardware](benchmarks/2026-10-03/desktop-hardware-pyramid/) nas condições acima. Todas passaram em todos os cenários.

| Modo de efeitos | Quadros em 10 s | CPU do servidor X | Mediana dos intervalos |
| --- | ---: | ---: | ---: |
| Xorg Xephyr, 1280×800: convolução → pirâmide | 85 → 598 | 85,8% → 20,4% | 116,681 → 16,699 ms |
| XLibre Xephyr, 1280×800: convolução → pirâmide | 85 → 599 | 86,1% → 19,4% | 116,640 → 16,655 ms |
| Hardware AMD/XLibre, 1920×1080: convolução → pirâmide | 49 → 599 | 93,7% → 4,1% | 199,998 → 16,667 ms |

Em hardware, cada MSC da fase ativa avançou uma unidade, e o Compust usou 0,4% de um núcleo. O desfoque elevou o Xorg de 3,1% na execução Present para 4,1%. Os resultados Present e diretos ficaram dentro da variação entre execuções dos registros anteriores. A pirâmide fica mais suave e um pouco mais forte que o filtro de caixa no raio 4.

## Sessões registradas em Intel/Xorg: 2026-10-03

Uma segunda máquina executou Linux Mint 22.3 com Linux 7.0.0-34-generic e Xorg 21.1.11 nativo, usando o driver modesetting com glamor. Ela tem um Intel Core i3-1005G1 com gráficos integrados Iris Plus G1 (Ice Lake, driver de kernel i915, Mesa 25.2.8) e um painel de 1366×768, eDP-1, a 60,06 Hz. O Xmonad era o 0.17.2 com xmonad-contrib 0.17.1. Os [registros](benchmarks/2026-10-03/intel-xorg/) omitem as capturas das cenas, e o log do servidor teve o nome da máquina e a linha de comando do kernel removidos.

As execuções aninhadas e de monitores usaram o commit limpo `ffd0b13222fa63723e8d1dac649fb1e0388e040e` com o binário do compositor `28c8a0622cda431433f77f60571c4880609766d7d857bc4143d4e259f94e676c`. A sessão dedicada usou esse commit mais a [mudança registrada do tempo limite do Present](benchmarks/2026-10-03/intel-xorg/desktop/present/source.patch), com o binário `6b6c51f0e44aabb601c6fbd24bb1d43e341b8875e3f932f24fc5a2eb33f2ff0c`.

### Medição aninhada

O Xorg Xephyr 21.1.11 hospedado pelo Xvfb em 1280×800 passou em todos os cenários nos três modos.

| Modo | CPU ativa do Compust | CPU ativa do Xephyr | Quadros em 10 s | Mediana / p95 dos intervalos |
| --- | ---: | ---: | ---: | ---: |
| [Present](benchmarks/2026-10-03/intel-xorg/nested/present/processes.csv) | 0,8% | 18,0% | 599 | 16,680 / 17,753 ms |
| [Direto](benchmarks/2026-10-03/intel-xorg/nested/direct/processes.csv) | 0,3% | 17,7% | 600 Damage | — |
| [Efeitos](benchmarks/2026-10-03/intel-xorg/nested/effects/processes.csv) | 0,9% | 32,4% | 594 | 16,749 / 17,846 ms |

### Transições de monitor no painel

O desktop Xmonad habitual ficou aberto com barra de status, bandeja, terminal e navegador. O [executor de transições de monitores](#amostrar-transições-de-monitores) amostrou a referência, o eDP-1 em 1280×720 e o modo restaurado, uma vez por modo de apresentação. As seis amostras passaram, e o Compust encerrou com sucesso após SIGTERM. A CPU é o percentual de um núcleo na fase ativa.

| Amostra | Raiz | Present: CPU do Compust / Xorg | Direto: CPU do Compust / Xorg | Bytes de pixmaps próprios |
| --- | --- | ---: | ---: | ---: |
| Referência | 1366×768 | 0,8% / 6,3% | 0,8% / 4,2% | 8.364.965 |
| eDP-1 em 1280×720 | 1280×720 | 0,8% / 6,4% | 0,7% / 4,1% | 7.358.677 |
| Modo restaurado | 1366×768 | 0,9% / 5,5% | 0,8% / 4,2% | 8.364.965 |

Cada [amostra Present](benchmarks/2026-10-03/intel-xorg/monitors/present/) recebeu 601 conclusões, todas `COPY`. Nos 1.800 intervalos da fase ativa, a mediana foi de 16,650 ms e o máximo de 16,671 ms, e cada MSC avançou uma unidade. As [amostras diretas](benchmarks/2026-10-03/intel-xorg/monitors/direct/) receberam de 707 a 711 notificações de Damage na janela de composição. Nas fases ociosas, o Compust usou 0,2–0,3% e o Xorg 0,9–1,4% de um núcleo, enquanto os próprios clientes do desktop causaram de 116 a 134 repinturas a cada dez segundos. As amostras de referência e de modo restaurado informaram `resources.csv` idênticos em cada modo. O RSS do Compust ficou entre 3.416 e 3.668 KiB, e o do Xorg foi de 63.916 a 64.468 KiB ao longo das duas execuções.

### Sessão dedicada de desktop

O [`hardware-session.sh`](../tools/hardware-session.sh) rodou em um novo servidor no vt3 enquanto a sessão habitual permaneceu no vt7. Os três modos passaram em todos os cenários; o [log da sessão](benchmarks/2026-10-03/intel-xorg/desktop/session.log) lista os resultados.

| Modo | CPU ativa do Compust | CPU ativa do Xorg | Quadros em 10 s | Mediana / p95 / máximo dos intervalos |
| --- | ---: | ---: | ---: | ---: |
| [Present](benchmarks/2026-10-03/intel-xorg/desktop/present/processes.csv) | 0,7% | 4,3% | 600 | 16,650 / 16,664 / 33,294 ms |
| [Direto](benchmarks/2026-10-03/intel-xorg/desktop/direct/processes.csv) | 0,5% | 3,0% | 600 Damage | — |
| [Efeitos](benchmarks/2026-10-03/intel-xorg/desktop/effects/processes.csv) | 0,7% | 4,4% | 600 | 16,651 / 16,664 / 33,294 ms |

As execuções Present e de efeitos tiveram, cada uma, um intervalo de dois vblanks; nos outros 598, o MSC avançou uma unidade. Nas fases ociosas, o Compust e o Xorg não registraram ticks de CPU. O RSS do Compust ficou em 3.580–3.664 KiB e o do Xorg em 99.804–99.812 KiB, sem mudança dentro de cada fase. O desfoque atrás da janela translúcida em tela cheia elevou o Xorg de 4,3% para 4,4% de um núcleo; portanto, a pirâmide também permanece na GPU com o glamor neste driver Intel.

Duas verificações informais não têm registro arquivado. O compositor da sessão habitual, usando Present com fades e desfoque padrão, continuou atualizando o relógio da barra de status depois de um DPMS-off forçado de oito segundos e, de novo, depois da troca para o vt3 e do retorno; ele não registrou nenhum tempo limite do Present.

Essas sessões usam as janelas sintéticas do probe, um painel, Xmonad 0.17.2 e fases de dez segundos. O laptop tem uma única tela, então a desconexão e a reconexão físicas não foram testadas nessas sessões; uma [sessão posterior](#sessão-registrada-de-hotplug-em-intelxorg-2026-10-03) acrescenta uma tela externa. Aplicativos reais, outros gerenciadores de janelas e taxas de atualização mistas continuam em aberto; um [registro posterior](#suspensão-e-retomada-registradas-2026-10-03) cobre a suspensão e a retomada.

## Sessão registrada de hotplug em Intel/Xorg: 2026-10-03

O [laptop Intel/Xorg](#sessões-registradas-em-intelxorg-2026-10-03) executou o [procedimento de transições de monitores](#amostrar-transições-de-monitores) com uma tela externa de 1920×1080 no HDMI-1, à direita do painel de 1366×768, em um desktop i3 4.23 habitual. O painel atualiza a 60,059 Hz e a tela externa a 60,000 Hz; portanto, este também é o primeiro registro com resoluções diferentes e taxas de atualização ligeiramente diferentes. A árvore era o commit limpo `0485ddcdec633859d3decca695238e2e13e411da`, com o binário do compositor `c3d810af11586206eff8c893d395a3b4a7c3f93bfcf43d71f27f8c7946d7bf31` e fades e desfoque desativados.

Os dois modos de apresentação passaram em onze amostras e encerraram com sucesso após SIGTERM: a referência, um modo de 1280×720 no HDMI-1 e sua restauração, um layout vertical e sua restauração, HDMI-1 desligado e ligado e, por fim, a desconexão física com o CRTC ainda atribuído, `xrandr --auto`, a reconexão e a restauração. A CPU é o percentual de um núcleo na fase ativa.

| Amostra | Raiz | Monitores ativos | Present: CPU do Compust / Xorg | Mediana dos intervalos do Present | Direto: CPU do Compust / Xorg | Bytes de pixmaps próprios |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Referência | 3286×1080 | 2 | 1,2% / 6,5% | 16,677 ms | 1,0% / 4,4% | 18.617.665 |
| HDMI-1 em 1280×720 | 2646×768 | 2 | 1,1% / 5,7% | 16,656 ms | 1,2% / 4,6% | 12.491.777 |
| Modo restaurado | 3286×1080 | 2 | 1,6% / 6,8% | 16,670 ms | 0,7% / 4,2% | 18.617.665 |
| Layout vertical | 1920×1848 | 2 | 1,4% / 6,8% | 16,668 ms | 0,6% / 4,2% | 18.614.785 |
| Layout restaurado | 3286×1080 | 2 | 1,5% / 6,5% | 16,668 ms | 0,6% / 4,1% | 18.617.665 |
| HDMI-1 desligado | 1366×768 | 1 | 1,2% / 6,0% | 16,632 ms | 0,7% / 4,2% | 8.441.857 |
| HDMI-1 ligado | 3286×1080 | 2 | 1,1% / 6,2% | 16,657 ms | 1,2% / 4,8% | 18.617.665 |
| HDMI-1 desconectado | 3286×1080 | 2 | 1,2% / 5,7% | 16,662 ms | 1,1% / 4,6% | 18.617.665 |
| `xrandr --auto` | 1366×768 | 1 | 1,5% / 6,3% | 16,647 ms | 1,2% / 4,5% | 8.441.857 |
| HDMI-1 reconectado | 1366×768 | 1 | 1,0% / 6,1% | 16,654 ms | 0,9% / 4,3% | 8.441.857 |
| Layout restaurado | 3286×1080 | 2 | 1,1% / 6,7% | 16,669 ms | 1,0% / 4,6% | 18.617.665 |

Cada amostra verifica repinturas do marcador perto de cantos opostos de cada monitor ativo e através da borda que os dois monitores compartilham. Toda topologia que ocorre mais de uma vez reproduziu o mesmo `resources.csv` em cada modo, incluindo a amostra desconectada em relação à referência. O Compust não registrou nenhum tempo limite do Present. Seu RSS foi de 3.432 a 3.476 KiB ao longo da [execução Present](benchmarks/2026-10-03/intel-xorg/hotplug/present/) e de 3.400 a 3.496 KiB ao longo da [execução direta](benchmarks/2026-10-03/intel-xorg/hotplug/direct/); o RSS do Xorg ficou em 96.616 KiB do início ao fim.

Nos 6.590 intervalos do Present das fases ativas, a mediana foi de 16,662 ms e o p95 pelo posto mais próximo de 16,679 ms. Três amostras tiveram, cada uma, um intervalo de dois vblanks; nas demais, cada MSC avançou uma unidade. A mediana foi de 16,632–16,654 ms só com o painel e de 16,656–16,677 ms com as duas saídas, o que é consistente com o Present seguindo um único CRTC, e não a atualização própria de cada monitor. Nas fases ociosas, o Compust usou no máximo 0,1% de um núcleo.

Esta sessão não detectou um defeito que o mesmo teste com o cabo mostrou no uso comum: janelas em tiling mantinham conteúdo antigo depois que o i3 as redimensionava e restaurava. Na época, o executor verificava apenas seu próprio marcador override-redirect, que um gerenciador de janelas não redimensiona; agora ele também repinta uma janela gerenciada e, com essa verificação, o binário da 0.2.0-beta.2 falha na amostra depois que o HDMI-1 é desligado. O [roteiro](ROADMAP.pt-BR.md#teste-local-da-beta-janela-parada-após-um-redimensionamento-restaurado) descreve o defeito e sua correção, que veio depois destes registros.

Foi testado um conector com uma tela externa, com Xorg e i3. As verificações do marcador comprovam repinturas no servidor, não o que cada tela exibiu. Taxas de atualização com diferença maior que essa, mais de dois monitores e fades ou desfoque durante uma transição continuam sem teste.

## Sessões registradas com Openbox: 2026-10-03

O [laptop Intel/Xorg acima](#sessões-registradas-em-intelxorg-2026-10-03) repetiu os três procedimentos com o Openbox 3.6.1, um gerenciador de janelas empilhadas que coloca cada cliente dentro de uma moldura decorada. A base foi o `bbfbd68489a5550bd9d3aa70c587244097d33028`, o commit da versão 0.2.0-beta.2, mais as [mudanças registradas do probe e do executor](benchmarks/2026-10-03/intel-xorg/openbox/desktop/present/source.patch); o código do compositor não mudou, e seu binário foi o `c3d810af11586206eff8c893d395a3b4a7c3f93bfcf43d71f27f8c7946d7bf31`. Os [registros](benchmarks/2026-10-03/intel-xorg/openbox/) seguem as mesmas convenções dos registros com Xmonad.

Todas as execuções passaram nos cenários existentes e nos três acrescentados para gerenciadores de janelas empilhadas: molduras decoradas, reempilhamento de janelas sobrepostas e minimização com restauração.

| Execução | CPU ativa do Compust | CPU ativa do servidor X | Quadros em 10 s | Mediana / p95 / máximo dos intervalos |
| --- | ---: | ---: | ---: | ---: |
| [Aninhada, Present](benchmarks/2026-10-03/intel-xorg/openbox/nested/present/processes.csv) | 0,8% | 13,4% | 599 | 16,690 / 17,804 / 19,867 ms |
| [Aninhada, direto](benchmarks/2026-10-03/intel-xorg/openbox/nested/direct/processes.csv) | 0,3% | 14,7% | 600 Damage | — |
| [Aninhada, efeitos](benchmarks/2026-10-03/intel-xorg/openbox/nested/effects/processes.csv) | 1,1% | 19,6% | 587 | 16,681 / 17,890 / 33,907 ms |
| [Hardware, Present](benchmarks/2026-10-03/intel-xorg/openbox/desktop/present/processes.csv) | 0,8% | 4,6% | 599 | 16,650 / 16,663 / 33,301 ms |
| [Hardware, direto](benchmarks/2026-10-03/intel-xorg/openbox/desktop/direct/processes.csv) | 0,6% | 2,9% | 600 Damage | — |
| [Hardware, efeitos](benchmarks/2026-10-03/intel-xorg/openbox/desktop/effects/processes.csv) | 0,7% | 4,7% | 600 | 16,650 / 16,663 / 33,291 ms |

O servidor aninhado é o Xorg Xephyr 21.1.11 em 1280×800; as linhas de hardware vêm de uma sessão dedicada no vt3 em 1366×768. Como com o Xmonad, cada execução Present em hardware teve um intervalo de dois vblanks. Nas fases ociosas em hardware, o Compust e o Xorg não registraram ticks de CPU, e o RSS não mudou dentro de cada fase: 3.380–3.440 KiB para o Compust.

O [executor de transições de monitores](benchmarks/2026-10-03/intel-xorg/openbox/monitors/) também passou na referência, no eDP-1 em 1280×720 e no modo restaurado, nos dois modos de apresentação, em um desktop Openbox habitual. Os intervalos do Present tiveram mediana de 16,65 ms e máximo de 16,67 ms, e cada MSC avançou uma unidade. Um terminal estava animando durante essas amostras, causando de 153 a 550 repinturas ociosas a cada dez segundos; por isso seus valores de CPU não são comparáveis aos das amostras com Xmonad. As amostras de referência e de modo restaurado da execução direta informaram `resources.csv` idênticos; as da execução Present não, porque o número de janelas abertas no desktop mudou entre as amostras.

Um comportamento do Openbox afetou o probe. Quando um cliente mapeava sua janela no momento em que o Compust assumia a tela, o Openbox 3.6.1 deixava o pedido de mapeamento sem tratamento até seu próximo evento; uma mudança posterior de propriedade na raiz o liberava. O probe agora envia essas mudanças de propriedade durante sua primeira espera sob um gerenciador de janelas empilhadas. A causa dentro do Openbox não foi investigada.

Essas sessões usam as janelas sintéticas do probe. Mover e redimensionar janelas de forma interativa, os menus do próprio Openbox e aplicativos reais não têm cenário registrado.

## Sessões registradas com i3: 2026-10-03

O mesmo laptop repetiu os três procedimentos com o i3 4.23, um gerenciador de janelas tiling que coloca cada cliente dentro de uma moldura com barra de título. A base foi o `025cabdd0cd690a960f1891613c9a28186a5cfdf` mais as [mudanças registradas do probe e do executor](benchmarks/2026-10-03/intel-xorg/i3/desktop/present/source.patch); o código do compositor não mudou, e seu binário foi o `c3d810af11586206eff8c893d395a3b4a7c3f93bfcf43d71f27f8c7946d7bf31`. Todas as execuções passaram em todos os cenários, incluindo as barras de título das molduras e a troca para o workspace da âncora.

| Execução | CPU ativa do Compust | CPU ativa do servidor X | Quadros em 10 s | Mediana / p95 / máximo dos intervalos |
| --- | ---: | ---: | ---: | ---: |
| [Aninhada, Present](benchmarks/2026-10-03/intel-xorg/i3/nested/present/processes.csv) | 0,8% | 18,3% | 599 | 16,679 / 17,776 / 18,984 ms |
| [Aninhada, direto](benchmarks/2026-10-03/intel-xorg/i3/nested/direct/processes.csv) | 0,3% | 17,9% | 600 Damage | — |
| [Aninhada, efeitos](benchmarks/2026-10-03/intel-xorg/i3/nested/effects/processes.csv) | 0,8% | 32,7% | 584 | 16,683 / 17,871 / 34,548 ms |
| [Hardware, Present](benchmarks/2026-10-03/intel-xorg/i3/desktop/present/processes.csv) | 0,7% | 4,4% | 599 | 16,650 / 16,664 / 33,296 ms |
| [Hardware, direto](benchmarks/2026-10-03/intel-xorg/i3/desktop/direct/processes.csv) | 0,4% | 2,9% | 600 Damage | — |
| [Hardware, efeitos](benchmarks/2026-10-03/intel-xorg/i3/desktop/effects/processes.csv) | 0,9% | 4,6% | 600 | 16,650 / 16,664 / 33,318 ms |

O servidor aninhado é o Xorg Xephyr 21.1.11 em 1280×800; as linhas de hardware vêm de uma sessão dedicada no vt3 em 1366×768. Cada execução Present em hardware teve um intervalo de dois vblanks. Nas fases ociosas em hardware, o Compust e o Xorg não registraram ticks de CPU, e o RSS do Compust ficou em 3.392–3.412 KiB.

O [executor de transições de monitores](benchmarks/2026-10-03/intel-xorg/i3/monitors/) passou na referência, no eDP-1 em 1280×720 e no modo restaurado, nos dois modos de apresentação, em um desktop i3 habitual com sua barra. A CPU da fase ativa foi de 0,6–0,8% para o Compust e 5,7–6,5% para o Xorg com Present, e de 0,6–0,7% e 3,9–4,2% com cópia direta. Os intervalos do Present tiveram mediana de 16,65 ms; duas das três amostras tiveram um intervalo de dois vblanks. As amostras de referência e de modo restaurado informaram `resources.csv` idênticos em cada modo.

Essas sessões usam as janelas sintéticas do probe no layout dividido padrão do i3. Os layouts empilhado e em abas, janelas flutuantes além da âncora e aplicativos reais não têm cenário registrado.

## Suspensão e retomada registradas: 2026-10-03

O [executor de transições de monitores](#amostrar-transições-de-monitores) também faz amostras em torno de uma suspensão: faça uma amostra, execute `systemctl suspend`, acorde a máquina e amostre de novo. No [laptop Intel/Xorg](#sessões-registradas-em-intelxorg-2026-10-03), em um desktop Openbox 3.6.1 habitual, cada modo de apresentação fez uma amostra antes de suspender para a RAM (`deep` em `/sys/power/mem_sleep`), uma cerca de seis segundos depois de acordar e mais uma depois que a segunda terminou. A árvore era o commit limpo `6f8d505ab55bd3db59605ca33a7b4fce95df6bdc`, com o binário do compositor `c3d810af11586206eff8c893d395a3b4a7c3f93bfcf43d71f27f8c7946d7bf31`. O relógio de parede avançou 197 segundos além do relógio monotônico na suspensão do modo Present e 54 segundos na do modo direto; o log do kernel não pôde ser lido para confirmar o estado de suspensão atingido.

| Amostra | Present: CPU do Compust / Xorg | Conclusões do Present, intervalos acima de um vblank | Direto: CPU do Compust / Xorg | Bytes de pixmaps próprios |
| --- | ---: | ---: | ---: | ---: |
| Antes de suspender | 1,2% / 8,7% | 600, 1 | 0,5% / 3,4% | 8.459.037 |
| Depois de retomar | 0,8% / 6,2% | 600, 0 | 0,8% / 4,4% | 8.459.037 |
| Depois de retomar, estabilizado | 0,9% / 8,5% | 598, 3 | 0,7% / 4,3% | 8.459.037 |

As seis amostras dos relatórios [Present](benchmarks/2026-10-03/intel-xorg/suspend/present/) e [direto](benchmarks/2026-10-03/intel-xorg/suspend/direct/) passaram nas verificações de repintura do marcador, e o Compust encerrou com sucesso após SIGTERM nas duas execuções. Os intervalos do Present tiveram mediana de 16,65 ms em todas as amostras. O Compust não registrou nenhum tempo limite do Present; portanto, os eventos de conclusão voltaram sozinhos aqui. Seu RSS foi de 3.248 a 3.336 KiB ao longo da execução Present e ficou em 3.344 KiB na execução direta.

Isso é uma suspensão por modo em uma máquina, com fades e desfoque desativados. Não cobre hibernação, suspensão com uma animação pendente nem outros drivers.

## Sessão registrada de recarga: 2026-10-03

Uma terceira máquina manteve um único processo do Compust durante três execuções do [executor de transições de monitores](#amostrar-transições-de-monitores) e recarregou sua configuração com SIGUSR1 entre as amostras. Ela tem um AMD Ryzen 5 5600X e uma Radeon RX 9060 XT (Navi 44, driver de kernel amdgpu, radeonsi no Mesa 26.2.4) e executa CachyOS com Linux 7.2.8 e XLibre 25.1.9 nativo com seu driver modesetting embutido. O dwm 6.8 gerenciava um desktop habitual em DP-2 a 1920×1080 e 60 Hz; HDMI-1, também a 1920×1080 e 60 Hz, foi ligado ao lado para as amostras com dois monitores. A base era `4d7b058b124dbac47df68f05424930891bda65ea` mais o [patch registrado](benchmarks/2026-10-03/reload-amd-dwm/source.patch) que adiciona a recarga, com o binário do compositor `87ab455db7f64f0c7fe20d0d2e836ea9a45c8676b2c83dca4b10dcfb0471f2b7`. O [executor](benchmarks/2026-10-03/reload-amd-dwm/runner.sh) segue `tools/hotplug-check.sh`, mas mantém o compositor em execução entre as execuções da sonda, com fases de três segundos.

| Passo antes da amostra | Raiz | Opções em uso | Mediana dos intervalos do Present | Bytes de pixmaps próprios |
| --- | --- | --- | ---: | ---: |
| Início, execução Present | 1920×1080 | Present, sem fade nem desfoque | 16,667 ms | 16.637.953 |
| HDMI-1 ligado, à direita de DP-2 | 3840×1080 | iguais | 16,667 ms | 25.093.633 |
| Recarga com `fade_ms = 120`, `blur_radius = 4` | 3840×1080 | desfoque 4 | 16,667 ms | 30.277.633 |
| Recarga com `opacity = 101`, rejeitada | 3840×1080 | inalteradas | 16,667 ms | 30.277.633 |
| HDMI-1 à esquerda de DP-2 | 3840×1080 | inalteradas | 16,667 ms | 30.277.633 |
| Recarga com `blur_radius = 16` | 3840×1080 | desfoque 16 | 16,667 ms | 30.602.113 |
| Recarga sem fade nem desfoque; HDMI-1 de novo à direita | 3840×1080 | sem fade nem desfoque | 16,667 ms | 25.093.633 |
| Recarga com `vsync = false`; execução direta da sonda | 3840×1080 | direta | — | 25.093.633 |
| Recarga com `blur_radius = 4` | 3840×1080 | direta, desfoque 4 | — | 30.277.633 |
| HDMI-1 desligado | 1920×1080 | direta, desfoque 4 | — | 19.229.953 |
| Recarga com `vsync = true`, sem desfoque; execução Present da sonda | 1920×1080 | Present, sem desfoque | 16,667 ms | 16.637.953 |
| HDMI-1 ligado, à direita de DP-2 | 3840×1080 | iguais | 16,667 ms | 25.093.633 |

As doze amostras passaram nas verificações do marcador em cada monitor e na borda compartilhada, e na repintura da janela que o dwm organizou lado a lado. A sonda também confere o caminho de apresentação: uma execução direta falha diante de qualquer conclusão do Present, e uma execução Present as exige; portanto, as recargas de `vsync` trocaram o caminho em hardware nos dois sentidos. Cada combinação de topologia e opções que aparece mais de uma vez reproduziu os mesmos bytes de pixmaps próprios depois de o renderizador ter sido substituído no intervalo. A recarga rejeitada registrou um aviso e manteve as opções em uso. Em 1.611 intervalos do Present nas fases ativas, a mediana foi 16,667 ms, o p95 por posição mais próxima 16,674 ms e o máximo 16,678 ms; todo MSC avançou de um em um. Nas fases ativas, o Compust usou no máximo 0,7% de um núcleo e o servidor X 7,7–8,0%, com os aplicativos do próprio desktop em execução; o RSS do Compust foi de 3.920 a 3.936 KiB. O Compust encerrou com sucesso após SIGTERM, e a configuração das saídas terminou como começou.

As janelas são as sintéticas da sonda. Nenhuma janela abriu ou fechou enquanto os fades estavam ativos, então nenhum fade atravessou uma recarga; recargas de `opacity` e `max_fps` e fades em andamento são cobertos apenas pelos [testes no Xvfb](../tests/cases/reload.rs). HDMI-1 foi ligado e desligado com `xrandr`, sem desconectar o cabo, e o log do servidor não pôde ser lido para confirmar a aceleração por glamor.

## Cenas de benchmark registradas: 2026-10-03

O Compust rodou duas vezes com os dois monitores e duas com somente DP-2 no [desktop com RX 9060 XT](#sessão-registrada-de-recarga-2026-10-03), com fases de 20 segundos no commit limpo `394b7e63404e8eac5028e7dc933f7458dd7fa696`. Nenhum aplicativo redesenhou durante as execuções. Os [registros brutos](benchmarks/2026-10-03/bench-amd-dwm/) preservam as medições originais; as tabelas abaixo apresentam as cargas do Compust.

Cada célula traz a CPU do Compust mais a do servidor X, em porcentagem de um núcleo e como média das duas execuções; as execuções concordam em até 0,25 ponto. Sem nada mudando na tela, o servidor usou 5,4%, a linha de base desta máquina. As cenas ativas mantiveram 60 quadros por segundo sem vblank perdido; a cena ociosa não apresentou quadros.

Dois monitores, 3840×1080:

| Cena | Compust  |
| --- | ---:  |
| Ociosa | 0,0 + 5,4%  |
| Janela pequena atualizando | 0,3 + 7,8%  |
| Translúcida em tela cheia | 0,3 + 8,0%  |
| Translúcida em tela cheia, desfoque | 0,4 + 8,2%  |
| Oito translúcidas | 0,4 + 8,5%  |
| Oito translúcidas, desfoque | 1,0 + 9,4%  |
| Mover e redimensionar | 1,1 + 8,8%  |
| Abrir e fechar | 0,4 + 9,1%  |

Somente DP-2, 1920×1080:

| Cena | Compust  |
| --- | ---:  |
| Ociosa | 0,0 + 5,4%  |
| Janela pequena atualizando | 0,2 + 7,4%  |
| Translúcida em tela cheia | 0,2 + 7,3%  |
| Translúcida em tela cheia, desfoque | 0,4 + 7,6%  |
| Oito translúcidas | 0,4 + 7,9%  |
| Oito translúcidas, desfoque | 1,0 + 8,8%  |
| Mover e redimensionar | 1,1 + 8,4%  |
| Abrir e fechar | 0,4 + 8,6%  |

O próprio processo do Compust usou 0,2–1,1% de um núcleo e 3.788–3.936 KiB de RSS. A cena da janela pequena marcou 8,4% de carga da GPU em dois monitores; essa versão repintava e copiava o quadro inteiro de 3840×1080 a cada mudança. A GPU muda o clock conforme a carga, então `gpu_busy_percent` mede o trabalho apenas de forma aproximada. Na cena ociosa com dois monitores, o XRes informou 41,6 MB em pixmaps do Compust; o XRes não inclui buffers GL.

Em cada execução, 46–58 das 100 novas janelas apareceram um quadro depois do pedido de mapeamento (16,3–16,7 ms), e as demais depois de dois (cerca de 33,2 ms). Essa divisão explica a variação da mediana de abertura; `latency.csv` lista cada ciclo. Janelas fechadas sumiram um quadro depois do pedido, em até 17,5 ms. As [mudanças posteriores no redesenho](#mudança-registrada-no-redesenho-2026-10-03) eliminaram o quadro redundante.

Esse registro cobre uma máquina com GPU dedicada rápida e monitores de 60 Hz, janelas sintéticas e um fundo cobrindo o desktop. Não cobre tela 4K, GPU lenta, Xorg nem o laptop Intel/Xorg. As latências incluem a leitura do overlay pela sonda e não medem os painéis.

## Mudança registrada no caminho de eventos: 2026-10-03

Depois da [etapa 2 do marco de renderização](ROADMAP.pt-BR.md#2-idas-e-voltas-no-caminho-de-eventos), o executor de benchmarks repetiu as cenas de janela pequena, de mover e redimensionar e de abrir e fechar com o Compust no mesmo desktop, duas vezes com os dois monitores e duas com somente DP-2, no commit limpo `5f088b6cf48041815ac0a830b5209e9fa87c1407`. Os [registros](benchmarks/2026-10-03/event-path-amd-dwm/) seguem os [anteriores](#cenas-de-benchmark-registradas-2026-10-03), que formam a coluna "antes". Cada célula traz a CPU do Compust mais a do servidor X, como média de duas execuções.

| Cena | Monitores | Antes | Depois |
| --- | --- | ---: | ---: |
| Mover e redimensionar | Dois | 1,08 + 8,78% | 0,95 + 8,65% |
| Mover e redimensionar | Um | 1,05 + 8,35% | 0,93 + 8,20% |
| Janela pequena atualizando | Dois | 0,30 + 7,75% | 0,28 + 8,05% |
| Janela pequena atualizando | Um | 0,25 + 7,40% | 0,28 + 7,38% |
| Abrir e fechar | Dois | 0,36 + 9,09% | 0,25 + 9,22% |
| Abrir e fechar | Um | 0,37 + 8,64% | 0,24 + 8,64% |

Mover e redimensionar custou ao Compust 0,13 ponto a menos e ao servidor 0,13–0,15 ponto a menos nas duas disposições. A cena redimensiona a janela a cada quadro, então cada evento ainda a recaptura, o que leva cerca de dez idas e voltas; a mudança remove apenas as consultas de árvore, geometria e formato. Um movimento puro agora não custa nenhuma, como a [regressão](../tests/cases/event_path.rs) verifica. As outras diferenças ficam dentro da variação entre execuções.

Na primeira execução com dois monitores, a cena da janela pequena perdeu 17 vblanks em menos de meio segundo. Ali o MSC do Present saltou 2^24 + 5 e depois avançou 4 em um tempo de oito vblanks, compatível com o Present trocando entre os CRTCs dos dois monitores, cujos contadores têm bases diferentes. O overlay cobre os dois monitores por igual, e a saída primária, DP-1, está desconectada. O Compust não registrou nenhum tempo limite do Present, e o número do servidor nessa execução, 8,25%, eleva a média com dois monitores acima. Nenhuma outra cena destes registros ou dos anteriores mostra uma descontinuidade. A sonda agora [as conta separadamente](#executar-as-cenas-de-benchmark); o `summary.csv` desta execução, escrito antes dessa mudança, informa o salto bruto de MSC como 16.777.229 vblanks perdidos.

## Mudança registrada no redesenho: 2026-10-03

Os [registros de benchmark](#cenas-de-benchmark-registradas-2026-10-03) mostraram cerca de metade das novas janelas um quadro mais tarde que as demais com o Compust. Uma versão instrumentada temporariamente registrou cada evento e envio do Present durante a cena de abrir e fechar. Em todos esses ciclos, o `UnmapNotify` e o `DestroyNotify` da janela foram lidos em lotes separados: o desmapeamento sozinho retirava a janela e seu quadro era enviado; depois, a destruição pedia outro quadro idêntico. Esse quadro saía assim que o primeiro terminava, e a janela seguinte, mapeada logo depois, esperava um vblank por ele, porque o único buffer do Present só é reutilizado depois que cada envio termina.

O Compust agora só redesenha para eventos que alteram uma superfície exibida ou a ordem delas. No commit limpo `338bb0777e2fe1ab87986ea71d130393a7ce70b2`, as mesmas três cenas rodaram duas vezes por disposição de monitores. Todas as 400 novas janelas apareceram um quadro depois do pedido de mapeamento, em no máximo 17,3 ms, contra 46–58 a cada 100 antes; os fechamentos continuaram levando um quadro. Os 100 ciclos levaram 3,33 segundos em vez de 4,1–4,3, com exatamente duas conclusões do Present cada. A CPU nas cenas de janela pequena e de mover e redimensionar ficou dentro da variação entre execuções, e nenhuma cena perdeu vblank. Os [registros](benchmarks/2026-10-03/repaint-amd-dwm/) seguem os anteriores.

## Repintura por regiões registrada: 2026-10-04

Depois da [etapa 3 do marco de renderização](ROADMAP.pt-BR.md#3-repintura-por-regiões), o executor mediu todas as cenas no mesmo desktop com o commit anterior `da928ad9f424736f28caf4a3317bcdf40de82a48` e o commit da repintura por regiões `a7efd319870ee808252bd649ebcdbac12007a4d3`, alternando as duas versões, duas vezes com os dois monitores e duas com somente DP-2. As duas versões tinham o código limpo e usaram o mesmo binário da sonda. Cada célula de CPU traz a CPU do Compust mais a do servidor X, como média de duas execuções; GPU é a ocupação média da amdgpu. Os [registros](benchmarks/2026-10-04/region-amd-dwm/) incluem a [sequência](benchmarks/2026-10-04/region-amd-dwm/sequence.sh) que os executou.

| Cena | Monitores | Antes | Depois | GPU antes | GPU depois |
| --- | --- | ---: | ---: | ---: | ---: |
| Ociosa | Dois | 0,00 + 5,97% | 0,00 + 5,35% | 0,1% | 0,0% |
| Ociosa | Um | 0,00 + 5,35% | 0,00 + 5,35% | 0,0% | 0,0% |
| Janela pequena atualizando | Dois | 0,30 + 8,03% | 0,28 + 7,30% | 8,6% | 7,5% |
| Janela pequena atualizando | Um | 0,30 + 7,45% | 0,28 + 7,30% | 7,8% | 7,0% |
| Translúcida em tela cheia | Dois | 0,32 + 8,03% | 0,30 + 8,00% | 9,4% | 9,6% |
| Translúcida em tela cheia | Um | 0,32 + 7,43% | 0,30 + 7,43% | 8,2% | 7,8% |
| Translúcida em tela cheia, desfoque | Dois | 0,53 + 8,45% | 0,55 + 8,47% | 10,8% | 11,0% |
| Translúcida em tela cheia, desfoque | Um | 0,50 + 7,82% | 0,53 + 7,78% | 8,8% | 8,6% |
| Oito translúcidas | Dois | 0,45 + 8,53% | 0,43 + 7,62% | 9,2% | 8,4% |
| Oito translúcidas | Um | 0,40 + 7,97% | 0,40 + 7,67% | 8,2% | 7,7% |
| Oito translúcidas, desfoque | Dois | 1,08 + 9,47% | 1,05 + 9,50% | 10,7% | 10,6% |
| Oito translúcidas, desfoque | Um | 1,10 + 9,07% | 1,12 + 8,90% | 9,6% | 9,1% |
| Mover e redimensionar | Dois | 1,00 + 8,68% | 0,97 + 8,22% | 8,5% | 7,5% |
| Mover e redimensionar | Um | 1,00 + 8,40% | 0,95 + 8,22% | 7,7% | 7,2% |
| Abrir e fechar | Dois | 0,45 + 9,30% | 0,45 + 8,70% | 8,9% | 8,6% |
| Abrir e fechar | Um | 0,40 + 9,10% | 0,45 + 8,55% | 7,8% | 7,2% |

Onde só parte da tela muda, o servidor X trabalhou menos. Com dois monitores, economizou 0,7 ponto com a janela pequena, 0,9 com oito janelas translúcidas e 0,5 ao mover e redimensionar. Abrir e fechar mediu 0,5–0,6 ponto a menos nas duas disposições, mas isso são dois ticks de CPU em uma cena de 3,3 segundos, como o [registro do reaproveitamento do desfoque](#reaproveitamento-do-desfoque-registrado-2026-10-04) explica. Com um monitor, em que o redesenho completo antigo cobria metade da área, a janela pequena economizou apenas 0,15 ponto, e mover e redimensionar, 0,2. A carga da GPU caiu 0,5–1,1 ponto nessas cenas, não pela metade: uma atualização de 64×64 ainda mantém a GPU 7–7,5% ocupada, então a maior parte dessa carga é um custo por quadro, não por pixel. Uma janela translúcida em tela cheia muda a tela inteira, e um dano sob qualquer uma de oito janelas desfocadas sobrepostas junta todas as suas áreas de alcance, então essas cenas repintam tanto quanto antes e custam o mesmo. A CPU do próprio Compust não variou mais de 0,05 ponto em nenhuma cena. Todas as cenas mantiveram 60 quadros por segundo; a versão antiga perdeu 4 vblanks uma vez com oito janelas desfocadas em um monitor, e a nova não perdeu nenhum.

A versão nova não apresentou nenhum quadro em nenhuma execução ociosa. A primeira execução ociosa da versão antiga, primeira cena da sequência, apresentou 20 quadros nos três primeiros segundos, enquanto clientes do desktop ainda redesenhavam depois que o compositor em uso foi parado; o número do servidor X nela, 6,55%, eleva a média ociosa dessa versão com dois monitores. Abrir e fechar foi a cena que mais variou entre execuções de uma mesma versão, até 0,8 ponto, menos de três ticks, para o servidor X: em uma execução da versão antiga, 19 dos primeiros ciclos levaram dois quadros para abrir e fechar, contra um quadro em todas as outras aberturas e fechamentos das duas versões.

O desfoque é agora o maior custo restante da cena de oito janelas. Com a repintura por regiões, ele acrescenta 0,6 ponto ao Compust e 1,9 ao servidor X com dois monitores, e 0,7 e 1,2 com um, porque uma mudança na janela de cima repinta toda a pilha de janelas desfocadas. É esse o custo que a [etapa 4](ROADMAP.pt-BR.md#4-oclusão-e-reaproveitamento-do-desfoque) reduziria.

## Reaproveitamento do desfoque registrado: 2026-10-04

Depois do [reaproveitamento do desfoque na etapa 4 do marco de renderização](ROADMAP.pt-BR.md#4-oclusão-e-reaproveitamento-do-desfoque), o executor mediu todas as cenas no mesmo desktop com o commit da repintura por regiões `fdc11224f3d3add229aa7197b6f00a79e5a9ea85`, cujo binário do compositor é idêntico à versão nova do [registro de regiões](#repintura-por-regiões-registrada-2026-10-04), e o commit do reaproveitamento `d1bd2f7f9ed7ef4d0e58cc71c76a308e3fb50040`. As versões se alternaram, duas vezes com os dois monitores e duas com somente DP-2; as duas tinham o código limpo e usaram o mesmo binário da sonda. As células se leem como no registro de regiões. Os [registros](benchmarks/2026-10-04/backdrop-amd-dwm/) incluem a [sequência](benchmarks/2026-10-04/backdrop-amd-dwm/sequence.sh).

| Cena | Monitores | Antes | Depois | GPU antes | GPU depois |
| --- | --- | ---: | ---: | ---: | ---: |
| Ociosa | Dois | 0,00 + 5,65% | 0,00 + 5,35% | 0,0% | 0,0% |
| Ociosa | Um | 0,00 + 5,35% | 0,00 + 5,32% | 0,0% | 0,0% |
| Janela pequena atualizando | Dois | 0,28 + 7,40% | 0,28 + 7,32% | 6,2% | 6,2% |
| Janela pequena atualizando | Um | 0,30 + 7,35% | 0,25 + 7,32% | 7,0% | 7,0% |
| Translúcida em tela cheia | Dois | 0,30 + 7,90% | 0,30 + 7,85% | 8,6% | 8,5% |
| Translúcida em tela cheia | Um | 0,28 + 7,40% | 0,30 + 7,38% | 7,7% | 7,8% |
| Translúcida em tela cheia, desfoque | Dois | 0,50 + 8,30% | 0,35 + 8,12% | 5,8% | 5,5% |
| Translúcida em tela cheia, desfoque | Um | 0,43 + 7,62% | 0,30 + 7,45% | 8,5% | 8,0% |
| Oito translúcidas | Dois | 0,40 + 7,62% | 0,40 + 7,65% | 6,5% | 6,5% |
| Oito translúcidas | Um | 0,40 + 7,62% | 0,43 + 7,70% | 7,5% | 7,5% |
| Oito translúcidas, desfoque | Dois | 1,02 + 9,22% | 0,50 + 8,00% | 8,6% | 6,5% |
| Oito translúcidas, desfoque | Um | 1,02 + 8,82% | 0,50 + 8,05% | 8,9% | 7,4% |
| Mover e redimensionar | Dois | 0,97 + 8,25% | 0,97 + 8,30% | 6,2% | 5,9% |
| Mover e redimensionar | Um | 0,97 + 8,28% | 0,97 + 8,28% | 7,1% | 7,0% |
| Abrir e fechar | Dois | 0,30 + 8,70% | 0,30 + 9,00% | 6,3% | 6,1% |
| Abrir e fechar | Um | 0,45 + 8,55% | 0,30 + 8,70% | 7,0% | 7,2% |

O reaproveitamento ajuda onde muda o próprio conteúdo de uma janela desfocada. Com oito janelas desfocadas sobrepostas e a de cima mudando, a CPU do Compust caiu pela metade, de 1,02% para 0,50%, e o servidor X trabalhou 1,2 ponto a menos com dois monitores e 0,8 com um. O desfoque agora acrescenta cerca de 0,1 ponto ao Compust e 0,35 ao servidor X em relação à mesma cena sem desfoque, contra 0,6 e 1,2–1,6 antes. Uma janela translúcida em tela cheia com desfoque custou 0,13–0,18 ponto a menos aos dois processos; cada quadro dela ainda copia o fundo guardado pela tela inteira. As cenas sem desfoque variaram no máximo dois ticks de CPU, exceto a ociosa com dois monitores: a primeira execução ociosa da versão antiga, de novo a primeira cena da sequência, apresentou 2 quadros e elevou essa média. Todas as cenas mantiveram 60 quadros por segundo, e nenhuma perdeu vblank.

Os números de CPU contam os ticks de escalonamento de cada processo, 100 por segundo. A maioria das cenas mede 20 segundos, em que um tick vale 0,05 ponto, mas abrir e fechar mede só 3,3 segundos, em que um tick vale 0,3 ponto. O número do servidor X nessa cena com dois monitores subiu exatamente um tick nas duas execuções, e a economia de 0,5–0,6 ponto que o registro de regiões informa para ela é de dois ticks.

A carga da GPU na cena de oito janelas com desfoque caiu de 8,6% para 6,5% com dois monitores e de 8,9% para 7,4% com um, mas entre cenas ela não acompanha o trabalho: a cena de tela cheia com desfoque mostrou menos carga que a mesma cena sem desfoque. Compare-a apenas entre versões dentro de uma mesma cena.

Os fundos guardados são pixmaps do servidor. Na cena de tela cheia com desfoque, os bytes de pixmaps que o XRes atribui ao Compust passaram de 71,5 MB para 96,4 MB: um fundo para a janela translúcida de 3840×1080 e outro, de 1920×1080, para a janela do Alacritty no desktop, que tem canal alfa e ficava sob o fundo opaco do benchmark. Na cena de oito janelas, passaram de 60,5 MB para 75,1 MB.

## Oclusão registrada: 2026-10-04

Depois da [oclusão na etapa 4 do marco de renderização](ROADMAP.pt-BR.md#4-oclusão-e-reaproveitamento-do-desfoque), o executor mediu todas as cenas, incluindo a nova cena `covered`, no mesmo desktop com o commit que acrescentou essa cena, `dc34c3b4e071a66fed23bc27ea35914b12bf446c`, cujo binário do compositor é idêntico à versão nova do [registro do reaproveitamento do desfoque](#reaproveitamento-do-desfoque-registrado-2026-10-04), e o commit da oclusão `899dfb64c3905095eb5a666a68a2d7ce89fbafef`. As versões se alternaram, duas vezes com os dois monitores e duas com somente DP-2; as duas tinham o código limpo e usaram o mesmo binário da sonda. As células se leem como no registro de regiões. Os [registros](benchmarks/2026-10-04/occlusion-amd-dwm/) incluem a [sequência](benchmarks/2026-10-04/occlusion-amd-dwm/sequence.sh).

| Cena | Monitores | Antes | Depois | GPU antes | GPU depois |
| --- | --- | ---: | ---: | ---: | ---: |
| Ociosa | Dois | 0,00 + 5,62% | 0,00 + 5,30% | 0,0% | 0,0% |
| Ociosa | Um | 0,00 + 5,32% | 0,00 + 5,35% | 0,0% | 0,0% |
| Janela pequena atualizando | Dois | 0,25 + 7,35% | 0,25 + 7,22% | 6,2% | 6,0% |
| Janela pequena atualizando | Um | 0,25 + 7,30% | 0,23 + 7,18% | 7,0% | 6,9% |
| Translúcida em tela cheia | Dois | 0,30 + 7,90% | 0,25 + 7,70% | 8,6% | 7,8% |
| Translúcida em tela cheia | Um | 0,28 + 7,35% | 0,25 + 7,22% | 7,7% | 7,2% |
| Translúcida em tela cheia, desfoque | Dois | 0,35 + 8,10% | 0,28 + 7,90% | 5,4% | 4,9% |
| Translúcida em tela cheia, desfoque | Um | 0,32 + 7,43% | 0,25 + 7,28% | 8,0% | 7,5% |
| Oito translúcidas | Dois | 0,40 + 7,68% | 0,38 + 7,55% | 6,5% | 6,6% |
| Oito translúcidas | Um | 0,38 + 7,72% | 0,38 + 7,62% | 7,5% | 7,3% |
| Oito translúcidas, desfoque | Dois | 0,47 + 8,07% | 0,45 + 8,18% | 6,5% | 7,1% |
| Oito translúcidas, desfoque | Um | 0,45 + 8,00% | 0,45 + 7,78% | 7,5% | 7,5% |
| Coberta | Dois | 0,45 + 8,45% | 0,25 + 7,82% | 8,8% | 4,2% |
| Coberta | Um | 0,43 + 7,95% | 0,23 + 7,18% | 8,1% | 6,9% |
| Coberta, desfoque | Dois | 0,50 + 8,85% | 0,25 + 7,80% | 5,5% | 4,2% |
| Coberta, desfoque | Um | 0,47 + 8,12% | 0,25 + 7,20% | 8,3% | 6,9% |
| Mover e redimensionar | Dois | 0,95 + 8,30% | 0,95 + 8,20% | 6,2% | 6,0% |
| Mover e redimensionar | Um | 0,95 + 8,28% | 0,95 + 8,20% | 7,0% | 7,0% |
| Abrir e fechar | Dois | 0,45 + 8,70% | 0,30 + 8,40% | 6,5% | 6,2% |
| Abrir e fechar | Um | 0,45 + 8,70% | 0,30 + 8,55% | 7,0% | 7,2% |

Pular janelas escondidas ajuda onde uma janela opaca cobre outras. Com oito janelas translúcidas sob uma janela opaca em tela cheia que muda a cada quadro, a CPU do Compust caiu de 0,43–0,50% para 0,23–0,25%, e o servidor X trabalhou 0,6–0,8 ponto a menos sem desfoque e 0,9–1,05 com ele; com dois monitores, a carga da GPU nessa cena caiu de 8,8% para 4,2%. A cena coberta agora custa o mesmo com desfoque e sem ele, porque as janelas desfocadas escondidas deixaram de ser desfocadas. Elas também descartam seus fundos, o que liberou 14,6 MB de pixmaps do servidor com dois monitores.

Em todas as outras cenas, o fundo opaco do benchmark agora esconde as janelas do próprio desktop e o fundo da raiz. Isso economizou até 0,2 ponto do servidor X e liberou o fundo de 8,3 MB que o registro do reaproveitamento encontrou guardado para a janela escondida do Alacritty. Oito janelas desfocadas custaram ao servidor X 0,1 ponto a mais com dois monitores nas duas execuções e 0,2 a menos com um, embora essa cena agora pule trabalho que antes pintava; abrir e fechar difere no máximo um tick. Todas as cenas mantiveram 60 quadros por segundo e nenhuma perdeu vblank. A primeira execução ociosa da versão antiga, de novo a primeira cena da sequência, apresentou quadros no início.

## Renderizador de GPU registrado: 2026-10-04

O [renderizador de GPU](ARCHITECTURE.pt-BR.md#renderizador-de-gpu) rodou no mesmo desktop no commit limpo `ab665d31`, com as entradas `compust` e `compust-gl` do executor de benchmarks lado a lado em cada execução, duas vezes com os dois monitores e duas com somente DP-2. O log dele nomeou o dispositivo usado: "AMD Radeon RX 9060 XT (radeonsi, gfx1200, ACO, DRM 3.64)". Os [registros](benchmarks/2026-10-04/gl-amd-dwm/) incluem a [sequência](benchmarks/2026-10-04/gl-amd-dwm/sequence.sh).

O modo de captura da sonda mostrou antes a mesma cena de três janelas translúcidas e uma opaca sobre listras de um pixel, com desfoque, em cada renderizador. As duas capturas de 3840×1080 diferem no máximo 2 níveis em qualquer canal: 76.896 pixels, 1,9% da tela, diferem 1, e 12 diferem 2. As duas [capturas](benchmarks/2026-10-04/gl-amd-dwm/compust-gl-blur.ppm.gz) estão nos registros.

Cada célula traz a CPU do Compust mais a do servidor X, como média de duas execuções. Todas as cenas mantiveram 60 quadros por segundo nos dois renderizadores, sem perder vblank. Na cena de abrir e fechar, agora com 20 segundos, cada uma das 2.400 janelas abertas pelo renderizador de GPU apareceu um quadro depois do pedido de mapeamento, em até 17,0 ms, como no XRender.

| Cena | Monitores | XRender | GPU |
| --- | --- | ---: | ---: |
| Ociosa | Dois | 0,00 + 5,40% | 0,00 + 5,35% |
| Ociosa | Um | 0,00 + 5,38% | 0,00 + 5,40% |
| Janela pequena atualizando | Dois | 0,25 + 7,28% | 0,65 + 6,97% |
| Janela pequena atualizando | Um | 0,23 + 7,22% | 0,68 + 7,00% |
| Translúcida em tela cheia | Dois | 0,30 + 7,82% | 0,75 + 7,32% |
| Translúcida em tela cheia | Um | 0,25 + 7,32% | 0,78 + 7,00% |
| Translúcida em tela cheia, desfoque | Dois | 0,28 + 7,85% | 0,78 + 7,35% |
| Translúcida em tela cheia, desfoque | Um | 0,28 + 7,40% | 0,75 + 6,97% |
| Oito translúcidas | Dois | 0,32 + 7,72% | 0,90 + 6,97% |
| Oito translúcidas | Um | 0,35 + 7,68% | 0,90 + 7,00% |
| Oito translúcidas, desfoque | Dois | 0,45 + 8,22% | 1,12 + 7,40% |
| Oito translúcidas, desfoque | Um | 0,47 + 7,85% | 1,10 + 6,97% |
| Coberta | Dois | 0,25 + 7,65% | 0,72 + 7,38% |
| Coberta | Um | 0,25 + 7,22% | 0,62 + 6,95% |
| Coberta, desfoque | Dois | 0,25 + 7,75% | 0,70 + 7,35% |
| Coberta, desfoque | Um | 0,25 + 7,25% | 0,68 + 6,97% |
| Mover e redimensionar | Dois | 0,95 + 8,28% | 2,02 + 9,15% |
| Mover e redimensionar | Um | 0,95 + 8,22% | 1,98 + 9,07% |
| Abrir e fechar | Dois | 0,60 + 7,84% | 1,42 + 8,12% |
| Abrir e fechar | Um | 0,57 + 8,59% | 1,38 + 8,87% |

O renderizador de GPU transfere trabalho do servidor X para o Compust. A CPU do próprio Compust subiu de 0,23–0,47% para 0,62–1,12% nas cenas que compõem janelas, e a do servidor X caiu 0,22–0,88 ponto; a soma ficou a até 0,25 ponto da do XRender. Mover e redimensionar custou 1,9 ponto a mais no total, e abrir e fechar, 1,1 a mais: cada novo pixmap de janela, um por redimensionamento, é compartilhado por DRI3, o que custa idas e voltas e, no glamor, pode exigir copiar o pixmap para um buffer que ele consiga compartilhar. A RSS do Compust subiu de cerca de 4 MiB para 66 MiB, quase tudo do driver GL. Neste desktop, portanto, o renderizador de GPU não economiza nada; um servidor X mais lento ou mais ocupado, em que o trabalho do XRender no servidor pesa mais, é onde ele poderia economizar.

Uma primeira execução, em `a137a7db`, falhou uma vez: na segunda execução com dois monitores, uma janela aberta pelo renderizador de GPU ficou preta até a sonda desistir. O glamor retém seus comandos de GPU até o servidor ficar ocioso ou uma fence ser acionada, então, com o servidor ocupado, a GPU podia ler uma janela recém-pintada, ou a cópia que compartilhá-la exigiu, antes de esses comandos serem enviados, e nenhum dano posterior a repintava. A correção aciona uma fence do SYNC antes de cada quadro; o teste opcional [`tests/gpu.rs`](../tests/gpu.rs) reproduz a corrida com outro cliente mantendo o servidor ocupado, o que deixou 4–14 de 100 pixmaps compartilhados desatualizados sem a fence e nenhum com ela. Os [registros da execução que falhou](benchmarks/2026-10-04/gl-first-amd-dwm/) foram mantidos.

## Sessões de desktop registradas na RX 9060 XT: 2026-10-04

O [desktop com RX 9060 XT](#sessão-registrada-de-recarga-2026-10-03) executou o [procedimento em hardware](#executar-as-verificações-de-desktop-em-hardware) com quatro gerenciadores de janelas. Cada sessão foi um servidor XLibre 25.1.9 dedicado no vt3, iniciado pelo `startx` enquanto a sessão habitual continuava no vt2, com a DP-2 como única saída ativa, em 1920×1080 e 60 Hz. Todos os [logs do servidor](benchmarks/2026-10-04/desktop-rx9060xt/xmonad/xorg.log) informam glamor sobre radeonsi com OpenGL 4.6 e TearFree ativado. As sessões com Xmonad 0.18.1, Openbox 3.6.1 e i3 4.25.1 rodaram o commit limpo `2311948f700930ee1f9f3b9ba9e6ec8e66c180cb`, que prepara a 0.3.0-beta.1. A sessão com bspwm 0.9.12 rodou o `526eb96`, dois commits depois, que muda o script e o probe e deixa o código do compositor como estava. Os binários do compositor foram duas compilações desse código: `d455cea47e220def…` com o Xmonad e `05961c098ae05c04…`, que o Cargo compilou junto com o probe, com os demais. Os [registros](benchmarks/2026-10-04/desktop-rx9060xt/) guardam os relatórios de cada sessão sem as capturas de tela.

Toda execução da tabela passou em todos os cenários.

| Gerenciador de janelas | Modo | CPU do Compust em atividade | CPU do servidor X em atividade | Quadros em 10 s | Intervalo: mediana / p95 / máximo |
| --- | --- | ---: | ---: | ---: | ---: |
| Xmonad | [Present](benchmarks/2026-10-04/desktop-rx9060xt/xmonad/present/processes.csv) | 0,2% | 1,6% | 600 | 16,667 / 16,674 / 16,677 ms |
| Xmonad | [Direto](benchmarks/2026-10-04/desktop-rx9060xt/xmonad/direct/processes.csv) | 0,1% | 1,5% | 600 Damage | — |
| Xmonad | [Efeitos](benchmarks/2026-10-04/desktop-rx9060xt/xmonad/effects/processes.csv) | 0,2% | 1,7% | 600 | 16,666 / 16,674 / 16,677 ms |
| Openbox | [Present](benchmarks/2026-10-04/desktop-rx9060xt/openbox/present/processes.csv) | 0,2% | 2,0% | 600 | 16,667 / 16,675 / 16,678 ms |
| Openbox | [Direto](benchmarks/2026-10-04/desktop-rx9060xt/openbox/direct/processes.csv) | 0,2% | 1,5% | 600 Damage | — |
| Openbox | [Efeitos](benchmarks/2026-10-04/desktop-rx9060xt/openbox/effects/processes.csv) | 0,3% | 2,1% | 600 | 16,667 / 16,675 / 16,678 ms |
| i3 | [Present](benchmarks/2026-10-04/desktop-rx9060xt/i3/present/processes.csv) | 0,3% | 2,1% | 600 | 16,667 / 16,675 / 16,677 ms |
| i3 | [Direto](benchmarks/2026-10-04/desktop-rx9060xt/i3/direct/processes.csv) | 0,2% | 1,6% | 600 Damage | — |
| i3 | [Efeitos](benchmarks/2026-10-04/desktop-rx9060xt/i3/effects/processes.csv) | 0,3% | 2,1% | 600 | 16,667 / 16,669 / 16,676 ms |
| bspwm | [Present](benchmarks/2026-10-04/desktop-rx9060xt/bspwm/present/processes.csv) | 0,3% | 1,6% | 600 | 16,667 / 16,675 / 16,677 ms |
| bspwm | [Direto](benchmarks/2026-10-04/desktop-rx9060xt/bspwm/direct/processes.csv) | 0,1% | 1,5% | 600 Damage | — |
| bspwm | [Efeitos](benchmarks/2026-10-04/desktop-rx9060xt/bspwm/effects/processes.csv) | 0,3% | 1,8% | 600 | 16,667 / 16,675 / 16,676 ms |

O Present acompanhou o vblank exatamente: todo MSC da fase ativa avançou em um nas oito execuções que usam o Present. Nas fases ociosas, o Compust e o servidor X não registraram ticks de CPU. O RSS do Compust ficou entre 4.128 e 4.276 KiB e o do servidor X entre 91.116 e 95.256 KiB, sem mudar dentro de cada fase. O modo de efeitos acrescenta fades de 180 ms, desfoque de raio 4 e a janela sobrevivente com 50% de opacidade; ele custou ao servidor X no máximo 0,2 ponto a mais que a execução com Present.

A sessão com o bspwm rodou duas vezes, e a tabela mostra a segunda. Na primeira, as execuções com Present e direta passaram e a de efeitos terminou: seus 600 quadros estão registrados, e o servidor X dela registrou um encerramento normal às 16:20:31. No minuto seguinte a tela ficou preta e a máquina parou de responder. Ela foi reiniciada à força, e os arquivos escritos nos últimos segundos daquela sessão nunca chegaram ao disco: a linha de resultado do probe, o registro de CPU e o fim do log da sessão estão vazios. Os logs não dizem por quê. O journal termina quando o servidor X seguinte começa a subir, e o histórico do shell mostra duas tentativas de `startx` nesses segundos, cada uma com um diretório de relatório que o script não consegue criar, o que o faz parar antes de iniciar o Compust. Os [registros da primeira sessão](benchmarks/2026-10-04/desktop-rx9060xt/bspwm-interrupted/) foram mantidos como estavam. A segunda sessão rodou quarenta minutos depois, sem incidentes.

O bspwm 0.9.12 às vezes mantém o espaço de uma janela que não existe mais: uma das 32 que o probe destrói logo depois do pedido de mapeamento. No Xvfb do Xorg isso ocorreu em 2 de 8 ensaios, nos quais o `xprop` confirmou que a janela do nó restante já não existia. Em hardware, ocorreu em uma das cinco execuções com bspwm cuja captura final sobreviveu: na execução com Present da primeira sessão, a janela sobrevivente ficou com metade da tela até o fim, e nas outras quatro ela ocupou a tela inteira. O probe aceita o espaço que sobra no bspwm, desde que ele mostre o fundo.

Uma primeira sessão com o Xmonad foi iniciada com `SESSION_OUTPUT=DP=2`. O `xrandr` ignorou o nome desconhecido e o script desligou as duas saídas, de modo que a sessão rodou em uma raiz de 320×200 sem nenhum CRTC ativo. Mesmo assim todos os cenários passaram: o Present concluía uma vez por segundo, e o Compust registrou sua recuperação de um segundo, "Present did not finish a submission; replacing its buffers", duas vezes na execução com Present e três na de efeitos. Os [registros](benchmarks/2026-10-04/desktop-rx9060xt/outputs-off/) guardam essa sessão como a única com todas as saídas desligadas, e o script agora para quando a saída indicada não está conectada.

Essas sessões usam as janelas sintéticas do probe, um monitor e fases de dez segundos. O dwm, usado na sessão do próprio desktop, não tem configuração privada no script, e hotplug físico, suspensão e retomada e o renderizador de GPU não fizeram parte delas. Com a mesma compilação em uma sessão comum deste desktop, o halo fosco em volta dos menus de contexto do Brave desapareceu, como pretende o [desfoque ponderado](ROADMAP.pt-BR.md#uso-cotidiano-regras-por-janela-e-desfoque-ponderado); essa conferência foi visual e não tem registro.

## Sombras, foco e suspensão em tela cheia registrados (2026-10-04)

O probe dedicado de recursos passou em XRender e GL no desktop AMD Ryzen 5 5600X / Radeon RX 9060 XT, com XLibre 25.1.9, Linux 7.2.8-arch1-1, modesetting com glamor e TearFree e radeonsi. Somente DP-2 estava habilitada, a 1920×1080 e 60 Hz, profundidade 24; HDMI-1 estava conectada, mas desabilitada. Não havia gerenciador de janelas: o próprio probe controlava `_NET_ACTIVE_WINDOW`. O [arquivo de registros](benchmarks/2026-10-04/features-rx9060xt/README.md) identifica a configuração, a base `1969270257af37c092bad5d440d86ea74e680c01` mais as alterações locais registradas, hashes dos fontes e dos executáveis, logs e capturas. Esses hashes coincidiram com os arquivos locais testados ao coletar o registro.

Todos os 2.073.600 pixels de cada cena de sombras passaram contra a referência independente com diferença de até um nível RGB, abaixo da tolerância de dois. O XRender diferiu em 40 pixels e o GL em 25. A [comparação entre pintores](benchmarks/2026-10-04/features-rx9060xt/features/gl/shadows-comparison.csv) encontrou 45 pixels diferentes, também a no máximo um nível. As duas imagens de foco foram idênticas entre pintores; passaram o foco nas duas direções, `NONE` e o comportamento sem a propriedade. O [log do compositor GL](benchmarks/2026-10-04/features-rx9060xt/features/gl/compust.log) confirma o pintor de GPU da RX 9060 XT sem volta ao XRender. As capturas PNG arquivadas preservam cada pixel RGB dos PPMs originais e foram inspecionadas visualmente.

Cada fase medida durou dez segundos após dois segundos de aquecimento e solicitou 600 redesenhos da janela inteira a 60 por segundo. As duas fases compostas registraram 600 conclusões Present e 600 eventos Damage do overlay; as duas suspensas registraram zero. O popup retomou a composição, a nova captura mostrou o que foi desenhado durante a suspensão e remover a janela que cobria a tela restaurou o fundo. Os dois processos do compositor encerraram com sucesso após SIGTERM.

| Pintor | Estado | CPU do Compust | CPU do servidor X | RSS do Compust antes/depois |
| --- | --- | --- | --- | --- |
| [XRender](benchmarks/2026-10-04/features-rx9060xt/features/xrender/processes.csv) | Composto | 0,3% | 1,9% | 4.236 / 4.236 KiB |
| XRender | Suspenso | 0,0% | 1,0% | 4.236 / 4.236 KiB |
| [GL](benchmarks/2026-10-04/features-rx9060xt/features/gl/processes.csv) | Composto | 0,6% | 1,4% | 71.112 / 71.112 KiB |
| GL | Suspenso | 0,0% | 1,1% | 71.112 / 71.112 KiB |

As porcentagens de CPU usam um núcleo. Com 100 ticks de contabilização por segundo, um tick em dez segundos é 0,1 ponto percentual; zero significa que o Compust não acumulou ticks naquele intervalo. A RSS ficou igual durante cada fase medida. Esse registro cobre cenas sintéticas curtas em uma tela. Ainda faltam registros de aplicativos reais em tela cheia, comportamento de foco de gerenciadores de janelas, outros drivers e servidores, vários monitores ativos, desempenho das sombras e estabilidade dos recursos em uso prolongado.


## Monitores com taxas diferentes registrados (2026-10-04)

O desktop em uso com RX 9060 XT / XLibre 25.1.9 / Xmonad 0.18.1 passou em nove amostras de monitores com HDMI-1 em 1920×1080 a 60 Hz e DP-2 em 1920×1080 a 180 Hz, formando uma raiz de 3840×1080. Antes do teste, DP-2 estava de fato a 60 Hz e foi alterado para 180 Hz. XRender com Present, XRender direto e GL com Present conferiram os cantos opostos dos dois monitores, a junção entre eles e o redesenho de uma janela gerenciada enquanto a saída principal mudava DP-2 → HDMI-1 → DP-2. O [registro](benchmarks/2026-10-04/mixed-refresh-rx9060xt/README.md) identifica o código em `a3e6f31`, binários, configurações, tempos brutos, CPU/RSS, XRes e estados das saídas.

Todos os pixels passaram, os três processos do compositor encerraram corretamente e o log GL confirmou renderização em hardware sem retorno ao XRender. As contagens XRes ficaram idênticas dentro de cada perfil, e os pixmaps mantiveram 24.962.725 bytes em todas as amostras. O relógio MSC/UST do Present acompanhou a saída principal, em cerca de 180 Hz ou 60 Hz; o maior intervalo entre conclusões ativas foi 16,675 ms. O binário e a configuração originais do compositor foram restaurados, deixando DP-2 principal a 180 Hz e HDMI-1 a 60 Hz.

Cada amostra mediu três segundos ociosos e três ativos após aquecimento. Os aplicativos continuaram desenhando, então as fases ociosas ainda tiveram quadros. O marcador pede apenas 60 atualizações/s e o teto do Compust era 120 qps; isso confere o funcionamento com taxas diferentes, sem certificar agendamento independente de saída a 180 qps nem latência do painel. O RSS GL subiu 2.640 KiB entre recriações do renderizador apesar das contagens XRes estáveis; três transições não estabelecem uma tendência prolongada de memória. Essa sessão não repetiu hotplug físico nem validou efeitos na disposição com taxas diferentes.

## Sessões Xorg registradas no desktop com Radeon Vega (2026-10-05)

O [desktop com Radeon Vega](#sessão-registrada-em-hardware-2026-10-03), um AMD Ryzen 5 5600GT cujos gráficos integrados (Cezanne) usam o radeonsi, passou do XLibre 25.1.9 para o Xorg 21.1.24 do CachyOS. Os dois servidores o controlam com modesetting e glamor, que o `/etc/X11/xorg.conf.d` desta máquina seleciona. O modesetting do Xorg 21.1.24 não tem TearFree, que as sessões XLibre tinham por padrão. Cada execução abaixo foi um servidor dedicado no vt3, iniciado por `startx` a partir de um console de texto enquanto a sessão i3 de costume continuava no vt2, com apenas a HDMI-1 ativa em 1920×1080 a 60 Hz e a DP-1 conectada, mas desativada. As duas rodaram o commit limpo `ed2cdb04f828887231660d4c10729245b4a80b73` no Linux 7.2.9-1-cachyos; o SHA-256 do compositor era `cc8ad9e1e74afaf9…` e o do probe, `5cbe6be39cbd9bd0…`. Antes disso, no servidor da sessão de costume, `compust --diagnose` informou as mesmas extensões que o XLibre, inclusive DRI3 1.2 e Present 1.2, e os testes de DRI3 do crate de GPU passaram nele com `COMPUST_GPU_DISPLAY=:0`; essas duas verificações não foram arquivadas.

O [procedimento em hardware](#executar-as-verificações-de-desktop-em-hardware) com o i3 4.25.1 passou em todos os cenários em cada modo, inclusive barras de título das molduras, o popup, a restauração da tela cheia, a volta à área de trabalho da âncora, a troca de papel de parede e o ciclo de vida rápido.

| Modo | CPU ativa do Compust | CPU ativa do servidor X | Quadros em 10 s | Intervalo mediana / p95 / máx. |
| --- | ---: | ---: | ---: | ---: |
| [Present](benchmarks/2026-10-05/desktop-vega-xorg/i3/present/processes.csv) | 0,4% | 2,9% | 600 | 16,667 / 16,667 / 16,667 ms |
| [Direto](benchmarks/2026-10-05/desktop-vega-xorg/i3/direct/processes.csv) | 0,3% | 2,5% | 600 Damage | — |
| [Efeitos](benchmarks/2026-10-05/desktop-vega-xorg/i3/effects/processes.csv) | 0,4% | 2,6% | 600 | 16,667 / 16,667 / 16,667 ms |

Todo MSC da fase ativa avançou em um nas duas execuções que apresentam. Nas fases ociosas, nenhum processo registrou tique de CPU. A fase ociosa da execução com efeitos teve uma conclusão do Present e um evento Damage no overlay, como dez das onze execuções com efeitos arquivadas antes em hardware; os outros dois modos não tiveram nenhum. A RSS do Compust ficou entre 4.336 e 4.364 KiB e a do servidor X entre 100.176 e 104.772 KiB, sem mudar dentro de cada fase, exceto por 132 KiB que o servidor ganhou na fase ativa da execução direta. Esses números não se comparam diretamente com a [sessão XLibre](#sessão-de-desktop-registrada-em-hardware-2026-10-03) desta máquina, que rodou um build anterior no Xmonad e com TearFree.

O [probe de recursos](benchmarks/2026-10-05/features-vega-xorg/README.md) passou em XRender e GL sem gerenciador de janelas. Os 2.073.600 pixels de cada cena de sombra passaram na referência independente com diferença de até um nível: 40 diferiram no XRender e 25 no GL, e os pintores diferiram em 45, as mesmas contagens da [RX 9060 XT](#sombras-foco-e-suspensão-em-tela-cheia-registrados-2026-10-04). As duas imagens de foco foram idênticas entre os pintores, e o foco nas duas direções, `NONE` e a alternativa sem a propriedade passaram. O log do GL indica `AMD Radeon Graphics (radeonsi, renoir, ACO, DRM 3.64, 7.2.9-1-cachyos)` e não registra volta ao XRender. As duas fases compostas registraram 600 conclusões do Present e 600 eventos Damage no overlay, e as duas fases suspensas nenhum; o popup retomou a composição com uma captura nova, e remover a cobertura restaurou o fundo.

| Pintor | Estado | CPU do Compust | CPU do servidor X | RSS do Compust antes/depois |
| --- | --- | --- | --- | --- |
| [XRender](benchmarks/2026-10-05/features-vega-xorg/features/xrender/processes.csv) | Composto | 0,5% | 3,4% | 4.372 / 4.372 KiB |
| XRender | Suspenso | 0,0% | 1,2% | 4.372 / 4.372 KiB |
| [GL](benchmarks/2026-10-05/features-vega-xorg/features/gl/processes.csv) | Composto | 1,1% | 1,7% | 66.180 / 66.180 KiB |
| GL | Suspenso | 0,0% | 1,3% | 66.180 / 66.180 KiB |

Os arquivos guardam os CSV, logs, configurações e hashes. Seus logs do servidor omitem a linha de comando do kernel e o EDID bruto e ocultam os números de série dos monitores, e as capturas do probe de recursos são cópias PNG sem perdas dos PPM originais. Essas sessões usam janelas sintéticas, um monitor e fases de dez segundos. Hotplug físico, outros gerenciadores de janelas, aplicativos reais e suspensão e retomada não têm registro neste servidor.

## Concluir os critérios de hardware

Use uma sessão de teste dedicada de Xorg ou XLibre com o gerenciador pretendido. Registre commit exato e hashes do build, distribuição, versão do servidor, GPU e driver, versão/configuração do gerenciador, `compust --diagnose`, `xrandr --verbose` e configuração do compositor. Pare o compositor existente antes de iniciar o Compust; guarde o comando para restaurá-lo. Não execute o probe de cenários no seu ambiente habitual de trabalho: ele cria e destrói janelas e troca workspaces. O modo de amostragem de monitores descrito acima move apenas o próprio marcador.

Execute a [sessão em hardware](#executar-as-verificações-de-desktop-em-hardware) para os cenários do probe, a troca de papel de parede, os fades e o desfoque com translucidez. Depois repita os cenários de ciclo de vida, menus, tela cheia e workspaces com aplicativos reais e acrescente gerenciadores com decoração ou reparenting e o encerramento da sessão. Registre os cenários aprovados, suas reproduções e os logs de cada falha. As verificações com Xmonad não validam outro gerenciador.

Para a etapa 2, execute o [procedimento de transições de monitores](#amostrar-transições-de-monitores) nos dois modos de apresentação em cada ambiente proposto para suporte. Ele registra nomes dos conectores, modos, taxas de atualização e disposição dos monitores em torno de cada desconexão e reconexão física, além de verificar atualizações sobre bordas compartilhadas entre monitores. Desativação/ativação de CRTC virtual está coberta na automação e não substitui esses testes de conectores. A sessão acima cobre um ambiente AMD/XLibre.

Para a etapa 3, meça CPU ociosa e ativa de Compust e servidor X, tendências de memória em operações repetidas e intervalos Present quando disponíveis. Inclua duração da carga, quantidade de janelas, opacidade/desfoque e taxas dos monitores. Repita a mesma carga e configuração do Compust para medir variação e regressões. Publique evidências para cada combinação servidor/gerenciador/driver antes de declará-la suportada; a distribuição da beta continua condicionada a esse escopo.
