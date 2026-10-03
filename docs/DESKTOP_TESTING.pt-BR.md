# Validação de desktops

[English (US)](DESKTOP_TESTING.md) | [Português (Brasil)](DESKTOP_TESTING.pt-BR.md)

Este guia cobre o trabalho reproduzível de desktops da etapa 3 da beta. O Xmonad dentro do Xephyr exercita um gerenciador de janelas e um servidor X reais. O Xephyr hospedado pelo Xvfb usa renderização por software; esses resultados não validam driver de GPU, monitor físico nem apresentação sem tearing. Uma sessão AMD/XLibre registrada abaixo cobre as transições físicas de monitores da etapa 2; outros hardwares e os cenários de desktop da etapa 3 em hardware continuam em aberto.

## Executar a medição isolada

Instale a toolchain Rust fixada pelo projeto, um linker C, GHC com as bibliotecas `xmonad` e `xmonad-contrib`, Xvfb, Xephyr, `xprop`, `xdpyinfo`, `xrandr` e utilitários Linux como `timeout`, `getconf` e `sha256sum`. Execute na raiz do repositório. O script aloca os dois displays automaticamente e inicia uma configuração privada do Xmonad; pode rodar em uma sessão Wayland ou sem desktop.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
cargo build --release --locked --bin compust --example desktop_probe
mkdir -p artifacts
tools/desktop-check.sh artifacts/desktop-present present
tools/desktop-check.sh artifacts/desktop-direct direct
```

Cada diretório de relatório precisa ser novo. Omitir o modo seleciona `present`; `--help` descreve o comando. Defina `XVFB` e `XEPHYR` com caminhos de executáveis alternativos para testar outro build de servidor, incluindo o Xephyr do XLibre. Eles não selecionam o display do seu desktop atual. `SECONDS_PER_PHASE` aceita 1–30 segundos e usa 10 por padrão. Execute as medições em sequência, sem outros builds ou benchmarks em andamento.

As duas configurações usam `opacity = 100`, `fade_ms = 0`, `blur_radius = 0` e `max_fps = 120`. A [configuração Present](../tools/desktop/compust.toml) usa `vsync = true`; a [configuração direta](../tools/desktop/compust-direct.toml) usa `vsync = false`. Esses resultados, portanto, não incluem os custos de fade, translucidez e desfoque.

O probe verifica organização das janelas pelo gerenciador, remoção de popup override-redirect, tela cheia via EWMH e restauração, troca para um workspace vazio e retorno, 32 sequências rápidas de criação/map/destruição e atualização de uma janela sobrevivente. Cada cena verifica pixels reais do overlay. O script também exige encerramento bem-sucedido do compositor após SIGTERM. A inicialização dos displays espera por `-displayfd`; seleção do compositor, gerenciador e cenas usam notificações X11 com prazo máximo.

Após dois segundos de aquecimento, o probe mede uma fase ociosa e outra com uma janela opaca grande alternando vermelho e azul a uma frequência solicitada de 60 atualizações por segundo. Ele registra separadamente CPU de Compust, Xephyr, Xvfb hospedeiro, Xmonad e probe, com RSS no início e no fim de cada fase. Os percentuais consideram um núcleo como 100%; zero significa que nenhum tick de CPU foi observado naquele intervalo. Valores de RSS nos extremos não constituem um teste prolongado de vazamentos.

No modo `present`, a fase ativa precisa receber várias conclusões Present do compositor. `frames.csv` contém timestamps UST do servidor, valores MSC, seriais e modos de conclusão. Calcule intervalos somente entre registros sucessivos da mesma fase. São tempos de conclusão por software, não latência entre entrada e exibição nem medições de atualização física do monitor.

No modo `direct`, o probe exige notificações Damage do overlay durante a atividade e rejeita qualquer conclusão Present do compositor. `frames.csv` contém apenas o cabeçalho: não há medição de regularidade via Present para cópia direta. Contagens Damage comprovam atividade de renderização, não a taxa de quadros exibidos. Este probe ainda exige a extensão Present do servidor para detectar um caminho selecionado incorretamente; o Compust de produção pode executar sem essa extensão.

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

## Concluir os critérios de hardware

Use uma sessão de teste dedicada de Xorg ou XLibre com o gerenciador pretendido. Registre commit exato e hashes do build, distribuição, versão do servidor, GPU e driver, versão/configuração do gerenciador, `compust --diagnose`, `xrandr --verbose` e configuração do compositor. Pare o compositor existente antes de iniciar o Compust; guarde o comando para restaurá-lo. Não execute o probe de cenários no seu ambiente habitual de trabalho: ele cria e destrói janelas e troca workspaces. O modo de amostragem de monitores descrito acima move apenas o próprio marcador.

Repita os cenários de ciclo de vida, menus, tela cheia e workspaces com aplicativos reais. Acrescente janelas decoradas/com reparenting, mudanças de papel de parede, encerramento da sessão, fades e janelas translúcidas com desfoque ativado. Registre os cenários aprovados, suas reproduções e os logs de cada falha. Os testes aninhados com Xmonad não validam outro gerenciador.

Para a etapa 2, execute o [procedimento de transições de monitores](#amostrar-transições-de-monitores) nos dois modos de apresentação em cada ambiente proposto para suporte. Ele registra nomes dos conectores, modos, taxas de atualização e disposição dos monitores em torno de cada desconexão e reconexão física, além de verificar atualizações sobre bordas compartilhadas entre monitores. Desativação/ativação de CRTC virtual está coberta na automação e não substitui esses testes de conectores. A sessão acima cobre um ambiente AMD/XLibre.

Para a etapa 3, meça CPU ociosa e ativa de Compust e servidor X, tendências de memória em operações repetidas e intervalos Present quando disponíveis. Inclua duração da carga, quantidade de janelas, opacidade/desfoque e taxas dos monitores. Compare com picom somente identificando versão/backend e usando cenas e efeitos equivalentes. Publique evidências para cada combinação servidor/gerenciador/driver antes de declará-la suportada; a distribuição da beta continua condicionada a esse escopo.
