# Roteiro de desenvolvimento

[English (US)](ROADMAP.md) | [Português (Brasil)](ROADMAP.pt-BR.md)

O objetivo é tornar este compositor mínimo em Rust uma opção prática para usuários de Xorg e XLibre. “Melhor que o picom” precisa significar melhorias observáveis em confiabilidade, latência, consumo de recursos ou manutenção. Reescrever um recurso em Rust, por si só, não comprova melhor desempenho.

## Base 0.1: implementada

O repositório contém um compositor executável com composição XRender, transições de opacidade, transparência alfa, desfoque por convolução, recorte por formato, acompanhamento de atualizações, empilhamento, tratamento de redimensionamento de janelas, propriedades de papel de parede e apresentação opcional por cópia com Present. A suíte verifica o comportamento de um servidor real no Xvfb. A documentação e as orientações de contribuição estão em inglês e pt-BR.

Esse marco cria uma base para experimentação. Ele não comprova compatibilidade completa com ambientes gráficos nem desempenho em hardware real.

## Em andamento: correção em ambientes reais

### Ciclo de vida de clientes e molduras: implementado

A associação do cliente agora acompanha criação tardia e remoção de `WM_STATE`, novos descendentes dentro de uma moldura existente, mudanças de parentesco entre molduras visíveis e a raiz, além da destruição do cliente. A opacidade da moldura mantém precedência. Um evento de opacidade pendente para um cliente já destruído não faz mais a moldura sobrevivente desaparecer.

Cinco testes de regressão de pixels em [client_lifecycle.rs](../tests/cases/client_lifecycle.rs) exercitam essas transições com o binário real do compositor. Os quatro cenários de ciclo de vida falharam antes da correção; desativar o novo tratamento de propriedades e a recuperação de `BadWindow` restrita ao cliente também reproduziu a opacidade desatualizada e o desaparecimento da moldura. A suíte completa agora contém 23 testes: cinco unitários, três de CLI e quinze de integração X11.

| Ambiente | Cobertura verificada | Evidência / limites |
| --- | --- | --- |
| Xvfb 21.1.24 no CachyOS, 320×240×24, XRender e Present 1.2 | Cinco regressões de ciclo de vida de clientes e molduras; suíte completa aprovada | Registro de 2026-10-02, `fade_ms = 0`, `blur_radius = 0` e vsync padrão nos casos de ciclo de vida. Os testes criam as hierarquias diretamente; não validam um gerenciador de janelas real nem uma GPU. |
| Xorg com gerenciador de janelas real e drivers Intel/AMD/NVIDIA | Pendente | Exige registro de servidor, gerenciador, driver, configuração e commit. |
| XLibre com gerenciador de janelas real e drivers Intel/AMD/NVIDIA | Pendente | Exige os mesmos registros de ambiente; os resultados no Xvfb não comprovam suporte. |

Reproduza as verificações com a toolchain fixada pelo repositório e o Xvfb instalado. As [execuções de CI](https://github.com/hashdefault/compust/actions/workflows/ci.yml) registram resultados para cada commit exato; inclua a revisão exibida abaixo nos relatos locais.

```sh
git rev-parse HEAD
cargo test --locked --test x11 client_lifecycle
```

Defina `XVFB=/caminho/para/Xvfb` se o servidor estiver fora de `PATH`.

### Critérios ainda pendentes

Teste Xorg e XLibre com gerenciadores de janelas reais e drivers Intel, AMD e NVIDIA. Registre servidor, driver, configuração e commit em cada relato. Cubra mudanças de parentesco após a inicialização, sequências rápidas de map/unmap/destroy, janelas decoradas e override-redirect, menus, tela cheia, ferramentas de papel de parede e encerramento da sessão.

Acrescente cobertura de redimensionamento RandR e hotplug reais, recuperação de falhas de apresentação e contagem de recursos durante redimensionamentos repetidos. Verifique formatos grandes ou fora da tela e propriedades malformadas. Amplie a cobertura de corridas com destruição para além da descoberta do cliente e das leituras de opacidade, sem esconder outros erros do X.

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
