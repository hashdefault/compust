# Roteiro de desenvolvimento

[English (US)](ROADMAP.md) | [Português (Brasil)](ROADMAP.pt-BR.md)

O objetivo é tornar este compositor mínimo em Rust uma opção prática para usuários de Xorg e XLibre. “Melhor que o picom” precisa significar melhorias observáveis em confiabilidade, latência, consumo de recursos ou manutenção. Reescrever um recurso em Rust, por si só, não comprova melhor desempenho.

## Base 0.1: implementada

O repositório contém um compositor executável com composição XRender, transições de opacidade, transparência alfa, desfoque por convolução, recorte por formato, acompanhamento de atualizações, empilhamento, tratamento de redimensionamento de janelas, propriedades de papel de parede e apresentação opcional por cópia com Present. A suíte verifica o comportamento de um servidor real no Xvfb. A documentação e as orientações de contribuição estão em inglês e pt-BR.

Esse marco cria uma base para experimentação. Ele não comprova compatibilidade completa com ambientes gráficos nem desempenho em hardware real.

## Próxima etapa: correção em ambientes reais

Teste Xorg e XLibre com gerenciadores de janelas reais e drivers Intel, AMD e NVIDIA. Registre servidor, driver, configuração e commit em cada relato. Cubra mudanças de parentesco após a inicialização, sequências rápidas de map/unmap/destroy, janelas decoradas e override-redirect, menus, tela cheia, ferramentas de papel de parede e encerramento da sessão.

Acrescente cobertura de redimensionamento RandR e hotplug reais, recuperação de falhas de apresentação e contagem de recursos durante redimensionamentos repetidos. Verifique criação tardia de `WM_STATE`, formatos grandes ou fora da tela e propriedades malformadas. Requisições podem coincidir com a destruição de uma janela; a correção deve tratar a corrida sem esconder outros erros do X.

**Aceitação:** reproduções documentadas viram testes quando viável; o uso normal não causa quedas nem deixa janelas invisíveis ou imagens antigas; mudanças repetidas de ciclo de vida não fazem os recursos do servidor crescerem indefinidamente. Mantenha uma matriz de compatibilidade com evidências.

## Depois: medir e reduzir o trabalho de renderização

Colete referências em builds de release para CPU ociosa, CPU do aplicativo e do servidor X, memória, regularidade dos quadros e latência entre entrada e exibição. Compare cenas e efeitos equivalentes com uma versão/backend registrados do picom. Inclua alta resolução e monitores com taxas diferentes.

Implemente regiões de dano, expansão das regiões de desfoque, descarte de áreas ocultas e cache somente quando a medição justificar a complexidade. Avalie múltiplos buffers de apresentação com controle explícito de propriedade e conclusão.

**Aceitação:** um benchmark reproduzível demonstra a melhora, os testes de pixels continuam corretos nas bordas das regiões e o trabalho ocioso não aumenta. Publique o benefício e as cargas em que ele deixa de existir.

## Backend e expansão do protocolo

Avalie um backend EGL/OpenGL ou Vulkan depois de especificar importação e sincronização. A integração moderna com X11 pode envolver DMA-BUF por DRI3, negociação de modificadores, agendamento por Present e sincronização explícita. Cada parte exige implementação real e testes de driver; consultas de versão não contam como suporte.

Gerenciamento de cores, HDR, VRR, extensões específicas do XLibre e agendamento por saída exigem investigação de protocolo e hardware antes de qualquer promessa. Wayland nativo está fora deste roteiro.

**Aceitação:** o backend importa conteúdo real sem cópia rotineira para a CPU, respeita o tempo de vida dos buffers, recupera-se das falhas previstas e apresenta cobertura de drivers e medições documentadas.

## Uso cotidiano

Adicione regras por janela com tipos definidos, descoberta e recarga de configuração, diagnóstico mais claro e empacotamento para distribuições. Amplie as animações para movimento e escala somente depois de resolver a interação com coordenadas de entrada e geometria do gerenciador. Considere sombras e cantos arredondados com tratamento correto de formato e dano.

**Aceitação:** o comportamento é configurável, documentado nos dois idiomas e testável, sem anunciar implicitamente compatibilidade com a configuração ou a linguagem de animação do picom.

Ainda não há datas de entrega. Abra uma issue para discutir uma mudança delimitada ou relatar uma falha observada; evite iniciar vários projetos de backend sobrepostos antes de alinhar os requisitos.
