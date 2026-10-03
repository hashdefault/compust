# Validação de desktops

[English (US)](DESKTOP_TESTING.md) | [Português (Brasil)](DESKTOP_TESTING.pt-BR.md)

Este guia cobre o trabalho reproduzível de desktops da etapa 3 da beta. O Xmonad dentro do Xephyr exercita um gerenciador de janelas e um servidor X reais. O Xephyr hospedado pelo Xvfb usa renderização por software; esses resultados não validam driver de GPU, monitor físico nem apresentação sem tearing. Sessões AMD/XLibre registradas abaixo cobrem as transições físicas de monitores da etapa 2 e os cenários de desktop do probe em hardware da etapa 3. Um laptop Intel/Xorg registrado cobre os mesmos cenários e uma mudança de modo em seu único painel. Outros hardwares, gerenciadores de janelas e aplicativos reais continuam em aberto.

## Executar a medição isolada

Instale a toolchain Rust fixada pelo projeto, um linker C, GHC com as bibliotecas `xmonad` e `xmonad-contrib`, Xvfb, Xephyr, `xprop`, `xdpyinfo`, `xrandr` e utilitários Linux como `timeout`, `getconf` e `sha256sum`. Execute na raiz do repositório. O script aloca os dois displays automaticamente e inicia uma configuração privada do Xmonad; pode rodar em uma sessão Wayland ou sem desktop. Defina `WINDOW_MANAGER=openbox` ou `WINDOW_MANAGER=i3` para testar o Openbox ou o i3, cada um com sua própria configuração privada ([Openbox](../tools/desktop/openbox.xml), [i3](../tools/desktop/i3.config)); isso exige o gerenciador escolhido instalado, e não GHC nem Xmonad.

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

O probe verifica organização das janelas pelo gerenciador, remoção de popup override-redirect, tela cheia via EWMH e restauração, troca para um workspace vazio, definição e remoção de um papel de parede na raiz nesse workspace, retorno, 32 sequências rápidas de criação/map/destruição e atualização de uma janela sobrevivente. Cada cena verifica pixels reais do overlay. Com o Openbox, as janelas mantêm seu tamanho dentro de molduras decoradas; por isso o probe também exige uma moldura com reparenting e barra de título pintada, sobrepõe as duas janelas e traz cada uma para a frente, e minimiza e restaura uma delas. Com o i3, um gerenciador tiling que também faz reparenting, o probe exige as mesmas barras de título nas molduras e mapeia antes uma pequena janela-âncora que o i3 envia para um segundo workspace, porque o i3 só mantém workspaces que têm foco ou alguma janela. O script também exige encerramento bem-sucedido do compositor após SIGTERM. A inicialização dos displays espera por `-displayfd`; seleção do compositor, gerenciador e cenas usam notificações X11 com prazo máximo.

Após dois segundos de aquecimento, o probe mede uma fase ociosa e outra com uma janela grande, opaca exceto no modo `effects`, alternando vermelho e azul a uma frequência solicitada de 60 atualizações por segundo. Ele registra separadamente CPU de Compust, servidor X, Xvfb hospedeiro quando houver, Xmonad e probe, com RSS no início e no fim de cada fase. Os percentuais consideram um núcleo como 100%; zero significa que nenhum tick de CPU foi observado naquele intervalo. Valores de RSS nos extremos não constituem um teste prolongado de vazamentos.

Nos modos `present` e `effects`, a fase ativa precisa receber várias conclusões Present do compositor. `frames.csv` contém timestamps UST do servidor, valores MSC, seriais e modos de conclusão. Calcule intervalos somente entre registros sucessivos da mesma fase. Em servidores aninhados, são tempos de conclusão por software; em hardware, acompanham o vblank do CRTC. Nenhum deles é latência entre entrada e exibição.

No modo `direct`, o probe exige notificações Damage do overlay durante a atividade e rejeita qualquer conclusão Present do compositor. `frames.csv` contém apenas o cabeçalho: não há medição de regularidade via Present para cópia direta. Contagens Damage comprovam atividade de renderização, não a taxa de quadros exibidos. Este probe ainda exige a extensão Present do servidor para detectar um caminho selecionado incorretamente; o Compust de produção pode executar sem essa extensão.

## Executar as verificações de desktop em hardware

[`tools/hardware-session.sh`](../tools/hardware-session.sh) executa os três modos com GPU e monitor físico. Ele é o cliente de um novo servidor X iniciado em um console de texto, então não substitui nem perturba uma sessão de trabalho. Por exemplo, pressione Ctrl+Alt+F3, faça login e execute:

```sh
cd caminho/para/compust
env SESSION_OUTPUT=HDMI-1 startx "$PWD/tools/hardware-session.sh" artifacts/hardware-desktop -- :20
```

Escolha um número de display livre e o nome de uma saída exibida pelo `xrandr`; sem `SESSION_OUTPUT`, é usada a primeira saída conectada. O script deixa somente essa saída ativa no modo preferido, desativa o apagamento da tela e mantém um cliente conectado para que o servidor não seja reiniciado entre as execuções. Ele registra saídas, provedores e GPU e depois executa `present`, `direct` e `effects` no novo servidor, continuando após um modo com falha. `WINDOW_MANAGER` também seleciona o gerenciador de janelas aqui. Por fim, copia o log do servidor quando ele pode ser lido e encerra, terminando a sessão. Não use teclado nem mouse até aparecer `Hardware session finished`; depois faça logout e volte à sua sessão habitual. Revise `xorg.log` antes de compartilhá-lo: ele inclui números de série dos monitores e a linha de comando do kernel.

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

`tools/bench.sh` mede cenas fixas com o Compust e depois com o picom em uma sessão X11 existente; ele não inicia servidor nem gerenciador de janelas. Pare antes o compositor da sessão e guarde o comando que o restaura. As [configurações](../tools/bench/) se equivalem: o Compust sem fades nem desfoque e com `vsync = true`, e o picom com as mesmas opções nos backends `xrender` e `glx`, sem sombras, fades, cantos arredondados, escurecimento nem suspensão da composição. As variantes com desfoque usam raio de 4 pixels. O backend xrender do picom não tem Dual Kawase e desfoca com uma caixa de tamanho 9; o backend glx usa Dual Kawase aproximando o mesmo tamanho. `COMPOSITORS` e `SCENES` escolhem um subconjunto.

```sh
cargo build --release --locked --bin compust --example desktop_probe
SERVER_PID=$(pgrep -x Xorg) WM_PID=$(pgrep -x dwm) tools/bench.sh artifacts/bench
```

Cada cena mapeia um fundo sobre toda a raiz e depois janelas override-redirect, para que nenhum gerenciador as posicione e a geometria seja igual em qualquer desktop. Depois de dois segundos de aquecimento, a sonda mede por `SECONDS_PER_PHASE`, dez segundos por padrão.

| Cena | Carga |
| --- | --- |
| `idle` | Somente o fundo |
| `small-update` | Uma janela de 64×64 alternando vermelho e azul 60 vezes por segundo |
| `fullscreen-translucent` | Uma janela a 50% cobrindo a raiz, alternando do mesmo modo |
| `eight-translucent` | Oito janelas de 480×360 sobrepostas a 50%; a de cima alterna |
| `move-resize` | Uma janela movida e redimensionada 60 vezes por segundo |
| `open-close` | 100 janelas, uma por vez, mapeadas até o overlay mostrá-las e destruídas até o overlay mostrar o fundo |

As cenas translúcidas rodam sem desfoque e, com o sufixo `:blur`, com ele. Todas cabem em uma tela de 1366×768; em uma maior, só a janela de tela cheia cresce.

Cada diretório de cena contém `summary.csv`, com intervalos do Present, CPU, RSS, latências de abertura e fechamento e carga da GPU; `frames.csv`, com cada conclusão do Present; `processes.csv`; `latency.csv` em `open-close`; `topology.txt`; `resources.csv`; e os logs do compositor e da sonda. A raiz do relatório reúne as linhas de resumo no próprio `summary.csv` e registra o commit, as configurações, a versão do picom e os hashes dos binários. A CPU é uma fração de um núcleo e, como nas outras medições, exclui o tempo de GPU. No amdgpu, a sonda também amostra `gpu_busy_percent` dez vezes por segundo; `GPU_BUSY` indica outro arquivo de carga. Uma latência de abertura ou fechamento vai da requisição até a sonda ler a mudança no overlay após uma conclusão do Present ou um evento Damage, então inclui uma ida e volta de `GetImage`. `skipped_vblanks` conta vblanks sem conclusão entre quadros consecutivos e só faz sentido em cenas que atualizam a cada vblank. Em uma raiz com vários monitores, o Present pode trocar o CRTC que acompanha, e o MSC do novo CRTC tem outra base; `msc_discontinuities` conta saltos de MSC incompatíveis com o tempo entre os quadros, cujos vblanks perdidos são estimados a partir desse tempo. O XRes conta pixmaps do X, mas não buffers GL, então os números do picom com glx subestimam sua memória.

O Present não conclui quadros enquanto o DPMS mantém os monitores desligados. O script os liga, desativa a proteção de tela e o DPMS durante a execução e restaura as opções anteriores ao final. Execute-o em uma sessão dedicada ou em um desktop ocioso: os redesenhos de outros aplicativos entram em todas as cenas, com qualquer compositor.

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

O [executor de benchmarks](#executar-as-cenas-de-benchmark) rodou duas vezes com os dois monitores e duas com somente DP-2 no [desktop com RX 9060 XT](#sessão-registrada-de-recarga-2026-10-03), com fases de 20 segundos. A árvore era o commit limpo `394b7e63404e8eac5028e7dc933f7458dd7fa696`, e o picom era a v13, revisão `d87a5ba`. Nenhum aplicativo redesenhou durante as execuções. Os [registros](benchmarks/2026-10-03/bench-amd-dwm/) contêm os quatro relatórios e o [script](benchmarks/2026-10-03/bench-amd-dwm/sequence.sh) que os executou.

Cada célula traz a CPU do próprio compositor mais a do servidor X, em porcentagem de um núcleo e como média das duas execuções; as execuções concordam em até 0,25 ponto sempre que a taxa de quadros se manteve. Sem nada mudando na tela, o servidor usou 5,3–5,4% com todos os compositores; essa parte de cada número do servidor é a linha de base desta máquina, não composição. Todas as cenas rodaram a 60 quadros por segundo sem vblank perdido, exceto onde uma taxa aparece.

Dois monitores, 3840×1080:

| Cena | Compust | picom xrender | picom glx |
| --- | ---: | ---: | ---: |
| Ociosa | 0,0 + 5,4% | 0,0 + 5,3% | 0,0 + 5,3% |
| Janela pequena atualizando | 0,3 + 7,8% | 0,7 + 7,4% | 1,4 + 6,6% |
| Translúcida em tela cheia | 0,3 + 8,0% | 0,8 + 7,8% | 1,5 + 6,6% |
| Translúcida em tela cheia, desfoque | 0,4 + 8,2% | 0,1 + 97,2%, 1,2 qps | 1,6 + 6,7% |
| Oito translúcidas | 0,4 + 8,5% | 1,6 + 10,1% | 1,8 + 6,7% |
| Oito translúcidas, desfoque | 1,0 + 9,4% | 0,3 + 90,9%, 4,6 qps | 3,3 + 6,7% |
| Mover e redimensionar | 1,1 + 8,8% | 1,1 + 8,3% | 2,8 + 9,0% |
| Abrir e fechar | 0,4 + 9,1% | 1,1 + 8,3% | 1,9 + 8,9% |

Somente DP-2, 1920×1080:

| Cena | Compust | picom xrender | picom glx |
| --- | ---: | ---: | ---: |
| Ociosa | 0,0 + 5,4% | 0,0 + 5,4% | 0,0 + 5,3% |
| Janela pequena atualizando | 0,2 + 7,4% | 0,7 + 7,4% | 1,4 + 6,5% |
| Translúcida em tela cheia | 0,2 + 7,3% | 0,8 + 7,8% | 1,4 + 6,6% |
| Translúcida em tela cheia, desfoque | 0,4 + 7,6% | 0,1 + 96,6%, 2,4 qps | 1,6 + 6,6% |
| Oito translúcidas | 0,4 + 7,9% | 1,6 + 10,1% | 1,8 + 6,5% |
| Oito translúcidas, desfoque | 1,0 + 8,8% | 0,3 + 92,7%, 3,9 qps | 3,1 + 6,6% |
| Mover e redimensionar | 1,1 + 8,4% | 1,1 + 8,3% | 2,7 + 8,9% |
| Abrir e fechar | 0,4 + 8,6% | 1,0 + 8,2% | 1,9 + 8,2% |

Onde o Compust é mais lento: nas cenas de atualização e translucidez, o servidor X trabalha mais com ele do que com o picom glx, de 0,7 a 2,7 pontos de um núcleo, porque o Compust renderiza com XRender dentro do servidor, enquanto o picom glx renderiza com OpenGL no próprio processo. A diferença é maior com desfoque sobre oito janelas. Somando os dois processos, o Compust fica no máximo 0,4 ponto acima do picom glx, com dois monitores e oito janelas translúcidas. Com uma janela de 64×64 atualizando em dois monitores, o amdgpu informou a GPU 8,4% ocupada com o Compust, 6,8% com o picom xrender e 3,8% com o picom glx: o Compust repinta e copia o quadro inteiro de 3840×1080 a cada mudança. A GPU escolhe o clock conforme a carga, então `gpu_busy_percent` compara o trabalho apenas de forma aproximada; as outras cenas a 60 quadros por segundo marcaram 3,9–10,6%, sem ordem consistente entre os compositores.

Onde o Compust é mais rápido: seu próprio processo usou 0,2–1,1% de um núcleo, menos que o picom glx em todas as cenas que mudam e menos que o picom xrender, exceto ao mover e redimensionar, em que ambos usaram 1,1%, e nas cenas com desfoque, em que o picom xrender desenhou poucos quadros. Somando os dois processos, o Compust fica 1,1–2,1 pontos abaixo do picom glx quando janelas se movem, mudam de tamanho, abrem e fecham. Seu RSS ficou em 3.788–3.936 KiB, contra 6.800–7.136 KiB do picom xrender e 79.232–82.124 KiB do picom glx, cujo número inclui o driver GL. Em cada execução, o Compust mostrou de 46 a 58 das 100 novas janelas um quadro depois do pedido de mapeamento (16,3–16,7 ms) e as demais depois de dois (cerca de 33,2 ms); o picom mostrou todas depois de dois quadros (32,4–34,5 ms). Essa divisão explica por que a mediana de abertura do Compust muda entre execuções; `latency.csv` lista cada ciclo. Todos os compositores retiraram cada janela fechada um quadro depois do pedido, em até 17,5 ms, e nenhum apresentou quadros com a tela ociosa.

O backend xrender do picom desfoca com um filtro de convolução do XRender, que o glamor executa na CPU, como a [sessão de desktop em hardware](#sessão-de-desktop-registrada-em-hardware-2026-10-03) constatou com o antigo desfoque em caixa do Compust. Com dois monitores, o desfoque em tela cheia caiu para 1,2 quadro por segundo enquanto o servidor X usava 97% de um núcleo. Na cena ociosa com dois monitores, o XRes informou 41,6 MB em pixmaps do Compust, 74,8 MB do picom xrender e 33,2 MB do picom glx, que também tinha quatro pixmaps GLX sem tamanho informado e buffers GL que o XRes não enxerga.

Trata-se de uma máquina com GPU dedicada rápida e monitores de 60 Hz, janelas sintéticas e um fundo cobrindo o desktop. Não cobre tela 4K, GPU lenta, Xorg nem o laptop Intel/Xorg. As latências incluem a leitura do overlay pela sonda e não dizem nada sobre os próprios painéis.

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

## Concluir os critérios de hardware

Use uma sessão de teste dedicada de Xorg ou XLibre com o gerenciador pretendido. Registre commit exato e hashes do build, distribuição, versão do servidor, GPU e driver, versão/configuração do gerenciador, `compust --diagnose`, `xrandr --verbose` e configuração do compositor. Pare o compositor existente antes de iniciar o Compust; guarde o comando para restaurá-lo. Não execute o probe de cenários no seu ambiente habitual de trabalho: ele cria e destrói janelas e troca workspaces. O modo de amostragem de monitores descrito acima move apenas o próprio marcador.

Execute a [sessão em hardware](#executar-as-verificações-de-desktop-em-hardware) para os cenários do probe, a troca de papel de parede, os fades e o desfoque com translucidez. Depois repita os cenários de ciclo de vida, menus, tela cheia e workspaces com aplicativos reais e acrescente gerenciadores com decoração ou reparenting e o encerramento da sessão. Registre os cenários aprovados, suas reproduções e os logs de cada falha. As verificações com Xmonad não validam outro gerenciador.

Para a etapa 2, execute o [procedimento de transições de monitores](#amostrar-transições-de-monitores) nos dois modos de apresentação em cada ambiente proposto para suporte. Ele registra nomes dos conectores, modos, taxas de atualização e disposição dos monitores em torno de cada desconexão e reconexão física, além de verificar atualizações sobre bordas compartilhadas entre monitores. Desativação/ativação de CRTC virtual está coberta na automação e não substitui esses testes de conectores. A sessão acima cobre um ambiente AMD/XLibre.

Para a etapa 3, meça CPU ociosa e ativa de Compust e servidor X, tendências de memória em operações repetidas e intervalos Present quando disponíveis. Inclua duração da carga, quantidade de janelas, opacidade/desfoque e taxas dos monitores. Compare com picom somente identificando versão/backend e usando cenas e efeitos equivalentes. Publique evidências para cada combinação servidor/gerenciador/driver antes de declará-la suportada; a distribuição da beta continua condicionada a esse escopo.
