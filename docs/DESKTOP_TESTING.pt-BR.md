# Validação de desktops

[English (US)](DESKTOP_TESTING.md) | [Português (Brasil)](DESKTOP_TESTING.pt-BR.md)

Este guia cobre o trabalho reproduzível de desktops da etapa 3 da beta. O Xmonad dentro do Xephyr exercita um gerenciador de janelas e um servidor X reais. O Xephyr hospedado pelo Xvfb usa renderização por software; esses resultados não validam driver de GPU, monitor físico nem apresentação sem tearing. O hotplug físico da etapa 2 e a validação de desktops em hardware da etapa 3 continuam em aberto.

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

## Concluir os critérios de hardware

Use uma sessão de teste dedicada de Xorg ou XLibre com o gerenciador pretendido. Registre commit exato e hashes do build, distribuição, versão do servidor, GPU e driver, versão/configuração do gerenciador, `compust --diagnose`, `xrandr --verbose` e configuração do compositor. Pare o compositor existente antes de iniciar o Compust; guarde o comando para restaurá-lo. Não execute o probe no seu ambiente habitual de trabalho: ele cria e destrói janelas e troca workspaces.

Repita os cenários de ciclo de vida, menus, tela cheia e workspaces com aplicativos reais. Acrescente janelas decoradas/com reparenting, mudanças de papel de parede, encerramento da sessão, fades e janelas translúcidas com desfoque ativado. Registre os cenários aprovados, suas reproduções e os logs de cada falha. Os testes aninhados com Xmonad não validam outro gerenciador.

Para a etapa 2, registre nomes dos conectores, resoluções, taxas de atualização e disposição dos monitores antes e depois de desconectar/reconectar fisicamente cada monitor externo. Exercite mudanças de disposição e resolução, incluindo janelas entre saídas, nos dois modos de apresentação. Verifique a continuidade das atualizações e os pixels restaurados após cada transição. Desativação/ativação de CRTC virtual já está coberta na automação e não substitui esses testes de conectores.

Para a etapa 3, meça CPU ociosa e ativa de Compust e servidor X, tendências de memória em operações repetidas e intervalos Present quando disponíveis. Inclua duração da carga, quantidade de janelas, opacidade/desfoque e taxas dos monitores. Compare com picom somente identificando versão/backend e usando cenas e efeitos equivalentes. Publique evidências para cada combinação servidor/gerenciador/driver antes de declará-la suportada; a distribuição da beta continua condicionada a esse escopo.
