# Arquitetura

[English (US)](ARCHITECTURE.md) | [Português (Brasil)](ARCHITECTURE.pt-BR.md)

O Compust é um cliente X11 que possui a seleção de compositor de uma tela. O servidor X continua responsável pela entrada, pelas conexões dos aplicativos e pelo hardware de exibição. O gerenciador de janelas continua responsável por posicionamento, decorações, foco e áreas de trabalho.

## Da janela ao quadro

```text
Aplicativo desenha em uma janela redirecionada
                     |
           Damage / eventos de janela
                     |
       Cena: empilhamento e tempo de vida
                     |
   XRender: fundo -> desfoque -> janelas
                     |
       Cópia com Present ou XRender
                     |
     Overlay do Composite (sem região de entrada)
```

`session.rs` negocia as capacidades e adquire `_NET_WM_CM_Sn`. Um bloqueio breve do servidor torna a verificação do proprietário e a aquisição do redirecionamento atômicas em relação aos outros clientes. O bloqueio também é liberado em caso de erro. Um timestamp real do servidor é obtido antes de adquirir a seleção e anunciar `MANAGER`.

`surface.rs` captura cada janela visível de nível superior em um pixmap nomeado. Damage é associado ao pixmap, que permanece válido até o término da animação de fechamento, mesmo que a janela original desapareça. `picture.rs` controla as imagens XRender e os pixmaps próprios; o encerramento da última conexão também libera recursos e redirecionamentos após falhas na inicialização.

`scene.rs` mantém as superfícies na ordem de empilhamento do servidor. `events.rs` atualiza geometria, tempo de vida, formato e opacidade. `surface/client.rs` descobre o aplicativo por `WM_STATE` em até oito níveis de descendentes. Cada janela visitada recebe uma assinatura de eventos de propriedade e de janelas filhas antes da leitura de seu estado. A criação tardia ou remoção de `WM_STATE`, a criação de filhos, as mudanças de parentesco e a destruição atualizam a associação e a opacidade da moldura afetada. A propriedade de opacidade da moldura tem precedência quando existe.

Um descendente que desaparece durante a descoberta é ignorado somente no caso de `BadWindow`. Se o cliente selecionado desaparecer antes da leitura de opacidade, a moldura sobrevivente usa sua própria opacidade ou o padrão opaco; os eventos seguintes de ciclo de vida atualizam a associação. Perder um cliente não deve encerrar a superfície da moldura. Trata-se de suporte limitado às convenções dos gerenciadores de janelas, não de uma implementação completa de EWMH/ICCCM.

## Renderização e agendamento

O renderizador desenha o papel de parede em um buffer reutilizável com a profundidade da raiz, compõe as janelas de baixo para cima e aplica uma máscara A8 para a opacidade efetiva. O alfa por pixel permanece na imagem de origem. Os retângulos de Shape recortam tanto a janela quanto o desfoque. O módulo de desfoque usa duas convoluções unidimensionais de caixa, com soma exata de coeficientes em ponto fixo igual a 65536.

`animation.rs` usa `Instant` monotônico e interpolação smoothstep inteira. O fechamento parte da opacidade amostrada naquele momento, preservando a continuidade de animações interrompidas. O loop agenda um quadro final no ponto de chegada, inclusive quando a duração é zero.

`compositor.rs` processa lotes limitados de eventos para que uma tempestade de eventos não adie indefinidamente a pintura. Há redesenho somente após dano, mudanças relevantes ou durante animações, respeitando `max_fps` como teto. Quando ocioso, o socket X é aguardado por no máximo um segundo para observar os sinais de encerramento. Damage ainda provoca redesenho da tela inteira; não é uma otimização por regiões.

Com Present, um único buffer é enviado no modo COPY e só volta a ser usado após as notificações de conclusão e liberação. Os IDs de assinatura distinguem eventos antigos depois da recriação dos buffers, e assinaturas anteriores são liberadas. É uma base simples, não um agendador de baixa latência com vários buffers. Sem Present, XRender copia diretamente para a janela de composição.

## Mapa do código

| Área | Arquivos |
| --- | --- |
| Inicialização e configuração | `main.rs`, `config.rs` |
| Propriedade e capacidades X11 | `session.rs`, `atoms.rs`, `capabilities.rs` |
| Recursos capturados e tempo de vida | `picture.rs`, `surface.rs` |
| Associação entre cliente e moldura | `surface/client.rs` |
| Cena e eventos | `scene.rs`, `events.rs`, `compositor.rs` |
| Efeitos e apresentação | `animation.rs`, `renderer.rs`, `renderer/{paint,blur,present,wallpaper}.rs` |
| Verificação com servidor real | `tests/x11.rs`, `tests/cases/`, `tests/support/` |

Os caminhos de código são relativos a `src/`, exceto os de teste. O crate usa `unsafe_code = "forbid"`; essa restrição vale para o próprio crate, não para os detalhes internos das dependências. Rust evita várias classes de erro de memória, mas corridas do protocolo, erros de renderização e vazamentos de recursos ainda exigem testes.

## Como estender

Um backend de GPU deve preservar os contratos da cena e dos tempos de vida ao substituir a renderização. Evite criar uma abstração genérica antes de entender os requisitos reais de importação, sincronização e apresentação. As consultas de versão de DRI3 e Sync servem apenas ao diagnóstico neste momento.

A próxima otimização deve partir de medições: regiões danificadas precisam incluir as dependências do desfoque, e o descarte de áreas ocultas precisa considerar transparência e formato. Meça também o servidor X, pois o XRender delega trabalho a ele. O [roteiro](ROADMAP.pt-BR.md) define critérios de aceitação para essas mudanças.
