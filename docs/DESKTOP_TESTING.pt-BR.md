# Validação de desktops

[English (US)](DESKTOP_TESTING.md) | [Português (Brasil)](DESKTOP_TESTING.pt-BR.md)

Este guia cobre o trabalho reproduzível de desktops da etapa 3 da beta. O Xmonad dentro do Xephyr exercita um gerenciador de janelas e um servidor X reais. O Xephyr hospedado pelo Xvfb usa renderização por software; esses resultados não validam driver de GPU, monitor físico nem apresentação sem tearing. Sessões AMD/XLibre registradas abaixo cobrem as transições físicas de monitores da etapa 2 e os cenários de desktop do probe em hardware da etapa 3. Um laptop Intel/Xorg registrado cobre os mesmos cenários e uma mudança de modo em seu único painel. Outros hardwares, gerenciadores de janelas e aplicativos reais continuam em aberto.

## Executar a medição isolada

Instale a toolchain Rust fixada pelo projeto, um linker C, GHC com as bibliotecas `xmonad` e `xmonad-contrib`, Xvfb, Xephyr, `xprop`, `xdpyinfo`, `xrandr` e utilitários Linux como `timeout`, `getconf` e `sha256sum`. Execute na raiz do repositório. O script aloca os dois displays automaticamente e inicia uma configuração privada do Xmonad; pode rodar em uma sessão Wayland ou sem desktop.

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

O probe verifica organização das janelas pelo gerenciador, remoção de popup override-redirect, tela cheia via EWMH e restauração, troca para um workspace vazio, definição e remoção de um papel de parede na raiz nesse workspace, retorno, 32 sequências rápidas de criação/map/destruição e atualização de uma janela sobrevivente. Cada cena verifica pixels reais do overlay. O script também exige encerramento bem-sucedido do compositor após SIGTERM. A inicialização dos displays espera por `-displayfd`; seleção do compositor, gerenciador e cenas usam notificações X11 com prazo máximo.

Após dois segundos de aquecimento, o probe mede uma fase ociosa e outra com uma janela grande, opaca exceto no modo `effects`, alternando vermelho e azul a uma frequência solicitada de 60 atualizações por segundo. Ele registra separadamente CPU de Compust, servidor X, Xvfb hospedeiro quando houver, Xmonad e probe, com RSS no início e no fim de cada fase. Os percentuais consideram um núcleo como 100%; zero significa que nenhum tick de CPU foi observado naquele intervalo. Valores de RSS nos extremos não constituem um teste prolongado de vazamentos.

Nos modos `present` e `effects`, a fase ativa precisa receber várias conclusões Present do compositor. `frames.csv` contém timestamps UST do servidor, valores MSC, seriais e modos de conclusão. Calcule intervalos somente entre registros sucessivos da mesma fase. Em servidores aninhados, são tempos de conclusão por software; em hardware, acompanham o vblank do CRTC. Nenhum deles é latência entre entrada e exibição.

No modo `direct`, o probe exige notificações Damage do overlay durante a atividade e rejeita qualquer conclusão Present do compositor. `frames.csv` contém apenas o cabeçalho: não há medição de regularidade via Present para cópia direta. Contagens Damage comprovam atividade de renderização, não a taxa de quadros exibidos. Este probe ainda exige a extensão Present do servidor para detectar um caminho selecionado incorretamente; o Compust de produção pode executar sem essa extensão.

## Executar as verificações de desktop em hardware

[`tools/hardware-session.sh`](../tools/hardware-session.sh) executa os três modos com GPU e monitor físico. Ele é o cliente de um novo servidor X iniciado em um console de texto, então não substitui nem perturba uma sessão de trabalho. Por exemplo, pressione Ctrl+Alt+F3, faça login e execute:

```sh
cd caminho/para/compust
env SESSION_OUTPUT=HDMI-1 startx "$PWD/tools/hardware-session.sh" artifacts/hardware-desktop -- :20
```

Escolha um número de display livre e o nome de uma saída exibida pelo `xrandr`; sem `SESSION_OUTPUT`, é usada a primeira saída conectada. O script deixa somente essa saída ativa no modo preferido, desativa o apagamento da tela e mantém um cliente conectado para que o servidor não seja reiniciado entre as execuções. Ele registra saídas, provedores e GPU e depois executa `present`, `direct` e `effects` no novo servidor, continuando após um modo com falha. Por fim, copia o log do servidor quando ele pode ser lido e encerra, terminando a sessão. Não use teclado nem mouse até aparecer `Hardware session finished`; depois faça logout e volte à sua sessão habitual. Revise `xorg.log` antes de compartilhá-lo: ele inclui números de série dos monitores e a linha de comando do kernel.

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
- mede dois segundos de aquecimento e fases ociosa e ativa de dez segundos, como na medição isolada, gravando `processes.csv` e `frames.csv`;
- grava em `resources.csv` as contagens de recursos XRes do compositor e o total de bytes dos pixmaps dele.

O probe não cria janelas gerenciadas nem troca workspaces, então aplicativos comuns podem continuar abertos; as atualizações deles entram na medição ociosa. As verificações de pixels leem o framebuffer do servidor X pelo overlay, não a luz emitida pelos painéis. O relatório omite os dados EDID de `xrandr --verbose` porque eles contêm números de série dos monitores. Os registros locais de processos ainda contêm linhas de comando, então revise o relatório antes de compartilhá-lo.

Uma sequência útil começa com uma referência inicial e depois muda o modo, a disposição e desativa uma saída, restaurando após cada mudança. Para cada conector, registre amostras com ele desconectado enquanto o CRTC ainda está atribuído, após a reação do desktop (`xrandr --auto` abaixo), reconectado e restaurado. Execute a sequência uma vez por modo de apresentação.

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

Essas sessões usam as janelas sintéticas do probe, um painel, Xmonad 0.17.2 e fases de dez segundos. O laptop tem uma única tela, então a desconexão e a reconexão físicas não foram testadas. Aplicativos reais, outros gerenciadores de janelas, suspensão e retomada e taxas de atualização mistas continuam em aberto.

## Concluir os critérios de hardware

Use uma sessão de teste dedicada de Xorg ou XLibre com o gerenciador pretendido. Registre commit exato e hashes do build, distribuição, versão do servidor, GPU e driver, versão/configuração do gerenciador, `compust --diagnose`, `xrandr --verbose` e configuração do compositor. Pare o compositor existente antes de iniciar o Compust; guarde o comando para restaurá-lo. Não execute o probe de cenários no seu ambiente habitual de trabalho: ele cria e destrói janelas e troca workspaces. O modo de amostragem de monitores descrito acima move apenas o próprio marcador.

Execute a [sessão em hardware](#executar-as-verificações-de-desktop-em-hardware) para os cenários do probe, a troca de papel de parede, os fades e o desfoque com translucidez. Depois repita os cenários de ciclo de vida, menus, tela cheia e workspaces com aplicativos reais e acrescente gerenciadores com decoração ou reparenting e o encerramento da sessão. Registre os cenários aprovados, suas reproduções e os logs de cada falha. As verificações com Xmonad não validam outro gerenciador.

Para a etapa 2, execute o [procedimento de transições de monitores](#amostrar-transições-de-monitores) nos dois modos de apresentação em cada ambiente proposto para suporte. Ele registra nomes dos conectores, modos, taxas de atualização e disposição dos monitores em torno de cada desconexão e reconexão física, além de verificar atualizações sobre bordas compartilhadas entre monitores. Desativação/ativação de CRTC virtual está coberta na automação e não substitui esses testes de conectores. A sessão acima cobre um ambiente AMD/XLibre.

Para a etapa 3, meça CPU ociosa e ativa de Compust e servidor X, tendências de memória em operações repetidas e intervalos Present quando disponíveis. Inclua duração da carga, quantidade de janelas, opacidade/desfoque e taxas dos monitores. Compare com picom somente identificando versão/backend e usando cenas e efeitos equivalentes. Publique evidências para cada combinação servidor/gerenciador/driver antes de declará-la suportada; a distribuição da beta continua condicionada a esse escopo.
