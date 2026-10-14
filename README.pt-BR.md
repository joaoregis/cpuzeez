<div align="center">

<img src="assets/wordmark.png" width="440" alt="CpuZeeZ">

**Reimplementação clean room, open source e multiplataforma das funcionalidades do CPU-Z, com monitor de hardware, escrita em Rust.**

Processador, caches, placa-mãe, memória e SPD, gráficos, sensores, discos, rede e bateria, benchmarks e relatórios TXT/HTML/JSON completos, no **Windows, Linux (Debian, Ubuntu, Arch, …) e macOS (Intel e Apple Silicon)**.

[English](README.md) · [Download](#instalação) · [Screenshots](#screenshots) · [Changelog](CHANGELOG.md) · [Cobertura](#cobertura-de-funcionalidades) · [Processo clean room](#processo-clean-room)

</div>

---

> O CpuZeeZ (`cpuzeez`) **não tem vínculo com a CPUID**. CPU-Z é marca registrada da CPUID. Nenhum código, tabela ou texto do CPU-Z foi usado. Veja [Processo clean room](#processo-clean-room).

## Destaques

- **Interface moderna.**
  - Navegação em barra lateral e um painel **Visão geral** ao vivo: temperaturas de CPU, GPU e chipset com mini-gráficos, além do estado de saúde e de limitação (throttling).
  - Páginas em cards, temas claro e escuro, com a fonte de interface do próprio SO.
  - Disponível em English, Português (Brasil) e Español.
- **Tudo o que o CPU-Z mostra.**
  - CPU e topologia (CCD/CCX, núcleos P/E/LP).
  - Caches, placa-mãe, chipset e BIOS.
  - Timings de memória: AMD pela UMC, Intel pelo MCHBAR, de Skylake a Arrow Lake.
  - SPD: de DDR a DDR5 e LPDDR4/5, com XMP, EXPO e EPP.
  - Gráficos, PCI, DMI, timers e dumps de registradores.
  - As mesmas 23 seções do `-txt` do CPU-Z, na mesma ordem.
- **Monitor de hardware.**
  - Todos os sensores com mínimo, máximo e média, gráficos ao vivo e log em CSV.
  - **Detecção de limitação** térmica, de potência e de corrente.
  - **Teste de estresse monitorado**, que para sozinho num limite de temperatura.
  - Ícone na bandeja / barra de menus com a temperatura da CPU, e um mini-monitor sempre visível.
  - Fontes de sensores:
    - **CPU:** temperaturas por núcleo, trilhos e PPT/TDC/EDC do SMU da AMD; RAPL Intel e AMD; clock efetivo por núcleo.
    - **GPU:** temperatura, hot spot, junção da memória, ventoinhas, potência, carga.
    - **Placa-mãe:** voltagens, temperaturas e ventoinhas do Super I/O, com mapas para mais de 150 modelos de placa.
    - **Chipset, DIMMs e discos.**
- **Benchmarks.**
  - CPU com uma e com várias threads, nas variantes Scalar, AVX2, AVX-512 e NEON.
  - **Latência e banda de memória e cache.**
  - **Histórico** local de benchmarks, para comparar antes e depois de um overclock ou de uma atualização de BIOS.
- **Dispositivos.** **SMART** completo para NVMe e SATA, **saúde da bateria** (desgaste, ciclos) e **adaptadores de rede** (link, IPs, DNS, driver, Wi-Fi).
- **Ferramentas.**
  - **Comparar relatórios**: o que mudou depois de uma troca de hardware ou de BIOS.
  - Clocks ao vivo de cada núcleo e da GPU, timers e exportação TXT/HTML/JSON.
- **A linha de comando do CPU-Z:**
  - As opções do CPU-Z: `-txt=`, `-html=`, `-json=`, `-console`, `-bench`, `-core=`, `-coremask=`.
  - Opções novas: `-sensors`, `-log=`, `-membench`, `-diff=`, `-checkupdate`.
  - Um `cpuzeez.ini` com as mesmas chaves do `cpuz.ini`.
- **Atualização em um clique.** O app encontra releases novas e instala o pacote certo para o seu SO, depois de conferir o SHA-256.
- **Um único binário nativo**, de cerca de 8 MB, sem dependências em tempo de execução.

## Screenshots

Geradas com `cpuzeez -screenshots=DIR`, que renderiza cada página e salva um PNG. As do Windows vêm de um AMD Ryzen 9 7900X, MSI PRO X870-P WIFI, 2×16 GB DDR5-6000 e uma RTX 4060, rodando como administrador com o PawnIO. Números de série, MACs e IPs ficam mascarados nesse modo.

### Windows 11

| Visão geral | Processador | Memória |
|---|---|---|
| ![Visão geral](docs/screenshots/windows/overview.png) | ![Processador](docs/screenshots/windows/processor.png) | ![Memória e SPD](docs/screenshots/windows/memory.png) |
| **Sensores** | **Teste de estresse** | **Benchmark de memória** |
| ![Sensores](docs/screenshots/windows/sensors.png) | ![Teste de estresse monitorado](docs/screenshots/windows/stress.png) | ![Benchmark de memória e cache](docs/screenshots/windows/membench.png) |
| **Dispositivos** | **Gráficos** | **Placa-mãe** |
| ![Dispositivos](docs/screenshots/windows/devices.png) | ![Gráficos](docs/screenshots/windows/graphics.png) | ![Placa-mãe](docs/screenshots/windows/mainboard.png) |
| **Histórico de benchmarks** | **Comparar relatórios** | **Preferências** |
| ![Histórico](docs/screenshots/windows/history.png) | ![Comparar relatórios](docs/screenshots/windows/compare.png) | ![Preferências](docs/screenshots/windows/settings.png) |
| **Tema claro, em português** | **Sobre, em português** | **Clocks** |
| ![Visão geral no tema claro, em português](docs/screenshots/windows/overview-light.png) | ![Sobre, em português](docs/screenshots/windows/about-pt.png) | ![Clocks](docs/screenshots/windows/clocks.png) |

### Linux

Ubuntu 24.04 (WSLg, Wayland), na mesma máquina, NVIDIA via NVML:

| Visão geral | Sensores | Dispositivos |
|---|---|---|
| ![Visão geral no Linux](docs/screenshots/linux/overview.png) | ![Sensores no Linux](docs/screenshots/linux/sensors.png) | ![Dispositivos no Linux](docs/screenshots/linux/devices.png) |

### macOS

Geradas pelo CI nos runners macOS do GitHub. São máquinas virtuais, então alguns campos mostram o que o hypervisor expõe:

| Apple Silicon: Visão geral | Apple Silicon: Processador | Mac Intel: Visão geral |
|---|---|---|
| ![Visão geral no macOS ARM](docs/screenshots/macos/overview.png) | ![Processador no macOS ARM](docs/screenshots/macos/processor.png) | ![Visão geral no Mac Intel](docs/screenshots/macos-intel/overview.png) |

## Instalação

Cada [release](https://github.com/joaoregis/cpuzeez/releases) traz um instalador por plataforma, o `SHA256SUMS.txt` e a seção do [changelog](CHANGELOG.md) daquela versão:

| Plataforma | Arquivo | Observações |
|---|---|---|
| Windows 10/11 x64 | `cpuzeez-<ver>-windows-x64.msi` | Instala para todos os usuários, cria atalho no menu Iniciar e adiciona o `cpuzeez` ao `PATH` |
| Windows (portátil) | `cpuzeez-<ver>-windows-x64-portable.zip` | Só o `cpuzeez.exe` |
| macOS 11+ (Intel e Apple Silicon) | `cpuzeez-<ver>-macos-universal.dmg` | Arraste o `CpuZeeZ.app` para Aplicativos |
| macOS (instalador) | `cpuzeez-<ver>-macos-universal.pkg` | Instala o app e o `/usr/local/bin/cpuzeez` |
| Debian 12+, Ubuntu 22.04+ | `cpuzeez_<ver>_amd64.deb` / `_arm64.deb` | `sudo apt install ./cpuzeez_<ver>_amd64.deb` |
| Arch Linux | `cpuzeez-bin-<ver>-1-x86_64.pkg.tar.zst` | `sudo pacman -U cpuzeez-bin-*.pkg.tar.zst` |
| Qualquer Linux (glibc 2.35+) | `cpuzeez-<ver>-linux-<arch>.tar.gz` | Binário, arquivo `.desktop` e ícones |

> **Sem certificado do publicador.** Os instaladores não têm assinatura com certificado do publicador, porque isso exige certificado pago. Os executáveis macOS usam assinatura ad-hoc, que não identifica o desenvolvedor nem fornece notarização da Apple.
> - **Windows:** o SmartScreen pode mostrar "O Windows protegeu o computador". Clique em *Mais informações → Executar assim mesmo*.
> - **macOS:** o Gatekeeper pode bloquear o app ou o `.pkg` porque a Apple não consegue verificar o desenvolvedor.
>   1. Confira o download com o `SHA256SUMS.txt`.
>   2. Tente abrir uma vez e feche o aviso.
>   3. Vá a *Ajustes do Sistema → Privacidade e Segurança → Abrir Mesmo Assim* e confirme *Abrir*.
>
>   Para o `.pkg`, conclua a instalação e repita para o app se aparecer outro aviso. No macOS 11–12, a opção fica em *Preferências do Sistema → Segurança e Privacidade → Geral*. Clicar com o botão direito → *Abrir* não contorna mais essa verificação no macOS 15 ou posterior. Veja as [instruções da Apple](https://support.apple.com/pt-br/102445).

Com Rust instalado, também dá para usar `cargo install --git https://github.com/joaoregis/cpuzeez`.

## Atualizações

**Aviso de atualização.** Ao abrir, o CpuZeeZ pergunta à API de releases do GitHub se existe versão mais nova. É um único `GET` anônimo; nada sobre o seu computador é enviado.
- Se houver, uma faixa oferece **Baixar e instalar**. Ela baixa o pacote do seu SO e da sua CPU, confere o SHA-256 com o `SHA256SUMS.txt` da release e então o abre:
  - **Windows:** o MSI.
  - **macOS:** o DMG ou o PKG.
  - **Debian/Ubuntu e Arch:** `pkexec` com `apt` ou `pacman`.
  - **Outros:** abre a pasta do download.
- **Novidades** abre as notas da release.
- Também dá para checar em *Sobre ▸ Procurar atualizações*, ou com `cpuzeez -checkupdate`.
- Para desligar, use `CheckUpdates=0` no `cpuzeez.ini`.
- O aviso usa o `curl` do sistema, que já vem no Windows 10+, no macOS e nas distros Linux desktop.

**Para atualizar manualmente**, basta instalar a versão nova por cima:

| Plataforma | Como |
|---|---|
| Windows | Rode o `.msi` novo. Ele atualiza no lugar: mesma pasta, um único item em *Aplicativos instalados*, PATH e atalho mantidos. Instalar uma versão mais antiga por cima de uma mais nova é recusado. O CI testa instalação → atualização → bloqueio de downgrade → desinstalação a cada push. |
| macOS | Arraste o `CpuZeeZ.app` novo por cima do antigo, ou rode o `.pkg` novo |
| Debian / Ubuntu | `sudo apt install ./cpuzeez_<ver>_amd64.deb` |
| Arch | `sudo pacman -U cpuzeez-bin-<ver>-1-x86_64.pkg.tar.zst` |

## Acesso privilegiado ao hardware

Como no CPU-Z, algumas informações só vêm de registradores do hardware: MSRs, o SMBus onde ficam as EEPROMs SPD, o chip Super I/O dos sensores, o controlador de memória e o SMU da AMD. O CPU-Z instala o próprio driver de kernel para isso. **O CpuZeeZ nunca traz driver.** Ele usa o que cada sistema já oferece e simplesmente omite o que não consegue ler.

| | Sem privilégios | Com privilégios |
|---|---|---|
| **Windows** | CPU, caches, topologia, DMI, BIOS, lista e links PCI, nomes do chipset, tipo/tamanho/canais da memória, SMART NVMe, rede, bateria, GPUs (NVML/NVAPI/ADL) com temperaturas e clocks, monitores, SO, timers, benchmarks | **+ PawnIO** (como administrador): clock real dos núcleos (APERF/MPERF), P-states, núcleos preferenciais, SPD, timings de memória, voltagens/temperaturas/ventoinhas do Super I/O, temperaturas e potência da CPU, SMU da AMD, temperatura do PCH Intel, temperatura das DIMMs, limites do SMART ATA |
| **Linux** | Tudo acima, mais sensores hwmon (`k10temp`, `coretemp`, `nct6775`, `it87`, `amdgpu`…), clocks via cpufreq, potência RAPL, e SPD quando `ee1004`/`spd5118` estão carregados | **+ root**: MSRs (`modprobe msr`), PCI config completo, SPD via `i2c-dev`, Super I/O via `/dev/port`, SMN/UMC da AMD, MCHBAR Intel, SMART via ioctl, tabela do SMU com o módulo opcional `ryzen_smu` |
| **macOS** | CPU (sysctl / CPUID), caches, clusters, SMBIOS (Intel) ou IODeviceTree (Apple Silicon), tamanho e tipo da memória, PCI (IOKit), GPU e Metal, monitores (EDID), discos, USB, rede, temperaturas/ventoinhas/potência do SMC, saúde da bateria, benchmarks | Nada a mais: o macOS não permite acesso de terceiros a registradores, e o Apple Silicon não tem SPD, MSRs nem Super I/O |

### Windows: PawnIO

O [PawnIO](https://pawnio.eu) é um driver de kernel assinado, open source e programável, também usado pelo LibreHardwareMonitor e pelo FanControl.

1. Instale: `winget install namazso.PawnIO`, ou baixe o instalador em [pawnio.eu](https://pawnio.eu).
2. Baixe os módulos oficiais ([releases do PawnIO.Modules](https://github.com/namazso/PawnIO.Modules/releases), LGPL-2.1). Extraia os `.bin` numa pasta `pawnio-modules` ao lado do `cpuzeez.exe`, ou na pasta de instalação do PawnIO.
3. Rode o CpuZeeZ **como administrador**. A página *Sobre* (e a seção *Binaries* do relatório) passa a mostrar `Hardware access: PawnIO 2.2.0 (modules: …)`.

Detalhes em [docs/windows-pawnio.md](docs/windows-pawnio.md).

### Linux: interfaces do kernel

```bash
sudo modprobe msr i2c-dev   # MSRs e SMBus
```
```bash
sudo modprobe spd5118       # SPD DDR5 (kernel 6.11+), ou ee1004 para DDR4
```
```bash
sudo cpuzeez                # detalhe completo; sem sudo você ainda vê quase tudo
```

## Uso

Rode `cpuzeez` sem argumentos para abrir o app. Todas as opções do CPU-Z funcionam, mais algumas novas:

| Opção | O que faz |
|---|---|
| `-txt=ARQ` | Relatório TXT completo (layout do dump do CPU-Z). O `.txt` é adicionado se faltar |
| `-html=ARQ` | Relatório HTML (com tema claro e escuro) |
| `-json=ARQ` / `-jsontxt=ARQ` | Relatório JSON (a mesma árvore do TXT) |
| `-console` | Imprime o relatório TXT no terminal |
| `-bench` | Benchmark de CPU com uma e com várias threads, salvo em `<hostname>.txt` e no histórico |
| `-membench` | Latência e banda de memória e cache (`-membench=quick` para uma rodada curta) |
| `-sensors` | Imprime todos os sensores uma vez, com o id de cada um |
| `-log=ARQ[,SEGUNDOS]` | Grava todos os sensores em CSV a cada segundo, até SEGUNDOS ou Ctrl+C |
| `-diff=ANTIGO.json,NOVO.json` | Compara dois relatórios (`NOVO` pode ser `now`; acrescente `,all` para incluir valores ao vivo) |
| `-core=N` | Mostra o clock do núcleo N e sai |
| `-coremask=HEX` | Restringe o `-bench` a essas CPUs lógicas |
| `-checkupdate` | Verifica se há release mais nova no GitHub |
| `-screenshots=DIR` | Renderiza cada página em PNG e sai (para docs e CI) |

Atalhos da interface, como no CPU-Z:
- **F5** salva a página atual em `.bmp`.
- **F6** copia a página para a área de transferência.
- **F7** salva um arquivo `.cvf` (o nosso relatório JSON).

### `cpuzeez.ini` e preferências

O `cpuzeez.ini` fica ao lado do executável e guarda as chaves de detecção de hardware. Ele aceita as mesmas chaves do `cpuz.ini`, e o arquivo UTF-16 do próprio CPU-Z é lido sem conversão.

As preferências da interface ficam na pasta de dados do usuário e são editadas na página **Preferências**:
- idioma, tema, ícone na bandeja, fechar para a bandeja;
- intervalo de leitura dos sensores, histórico dos gráficos, temperatura de alerta.

A pasta é `%APPDATA%\CpuZeeZ`, `~/Library/Application Support/CpuZeeZ` ou `~/.config/cpuzeez`. Ela também guarda o histórico de benchmarks (`history.json`) e os logs de sensores.

Para mapear os sensores de uma placa-mãe nova, `CPUZEEZ_EC_DUMP=1 cpuzeez -txt=placa` inclui no relatório os registradores brutos do Super I/O e as entradas sem mapa. Veja [docs/boards.md](docs/boards.md).

## Cobertura de funcionalidades

A especificação completa, montada a partir do relatório e do readme do próprio CPU-Z, está em [FEATURES.md](FEATURES.md). A tabela detalhada por SO está no [README em inglês](README.md#feature-coverage), e o que mudou em cada versão está no [CHANGELOG.md](CHANGELOG.md). Resumo:

- **Tudo o que se aplica a cada plataforma foi implementado:**
  - CPU, topologia, caches, dumps de CPUID e MSR, P-states, clocks.
  - SMBIOS/DMI, BIOS, chipset e Bus Specification, timings de memória AMD e Intel, clocks de uncore e fabric.
  - SMU da AMD: temperaturas por núcleo, L3 e SoC, trilhos VDDCR, PPT/TDC/EDC, Fmax e max ratio por núcleo.
  - SPD de todas as gerações, LPCIO, sensores de CPU, GPU, placa-mãe, chipset e DIMMs.
  - PCI, discos e SMART, bateria, rede, USB, GPUs (incluindo o fabricante da VRAM NVIDIA), monitores, NPUs, software, timers.
  - Relatórios, comparação de relatórios, benchmarks de CPU e memória, histórico, teste de estresse, log de sensores.
- **Impossível por limitação da plataforma:**
  - **macOS:** sem SPD (memória soldada), sem timings de memória, sem Super I/O e sem PCI config. Sem MSRs no Apple Silicon.
  - **Windows:** sem decodificação de capabilities PCI, porque nenhum módulo do PawnIO dá acesso genérico ao PCI config.
- **Ainda não decodificado:** os limites "fused" que o CPU-Z obtém por uma chamada de mailbox do SMU, e o escalar do PBO.
- **Fora de escopo de propósito:**
  - Validação online (valid.x86.fr, serviço da CPUID).
  - O algoritmo e os scores de referência do benchmark do CPU-Z. Os nossos benchmarks são próprios, e os scores não são comparáveis.
  - Alvos obsoletos: IA-64, Win9x, CPUs anteriores ao i686.

## Precisão: lado a lado com o CPU-Z

Na máquina de referência (Ryzen 9 7900X, MSI PRO X870-P WIFI, Windows 11 25H2), o `cpuzeez -txt` foi comparado seção por seção com o `cpuz.exe -txt`.

**Saíram iguais:**
- **CPU e topologia:**
  - Árvore APIC, lista de instruções, caches, microcode.
  - Codinome, pacote e stepping (`Raphael`, `Socket AM5 (LGA1718)`, `RPL-B2`).
  - P-states (`47.00x - 1.256 V`), núcleos preferenciais.
- **SMU da AMD:**
  - Versão `84.79.225`, `PMTV 00540004`, PPT/TDC/EDC 230 W / 160 A / 225 A, `Max Frequency 5700 MHz`.
  - Trilhos VDDCR_VDD/SOC/MISC e o layout das temperaturas por núcleo, L3 e SoC.
- **Chipset e memória:**
  - `Bus Specification PCI-Express 5.0 (32.0 GT/s)`.
  - Timings DDR5 `30-36-36-76`, tRC 112, razão 1:30, `Memory Max Frequency 2800.0 MHz`.
- **SPD:** os 2 × 1024 bytes lidos ao vivo são **idênticos byte a byte aos lidos pelo CPU-Z**, assim como todos os campos decodificados.
- **Super I/O:** registradores, voltagens, temperaturas e ventoinhas do NCT6687 (`Fan 15 SYS 2`, `Fan 16 SYS 1`).
- **DMI, discos, USB, GPUs e software:**
  - RTX 4060 `AD107-400`, 3072 cores, fabricante da memória `Samsung`, limites e clocks.
  - O EDID do monitor e a linha do SO.
- **Timers.**

**Diferenças intencionais:**
- **Tamanho da ROM do BIOS:** mostramos `32768 KB`, o campo estendido da especificação SMBIOS. O CPU-Z ignora esse campo e mostra `16384 KB`.
- **TMUs da RTX 4060:** mostramos `96`, a especificação pública (24 SMs × 4). O CPU-Z mostra 192.
- **Valores ao vivo** dependem do momento: temperaturas, clocks e max ratio por núcleo. Os ratios são medidos como o CPU-Z descreve (cada núcleo carregado sozinho), então variam com a temperatura.
- **Clock de referência:** medido por TSC ÷ razão base. Dá `Memory Frequency 2999.9 MHz`, onde o CPU-Z mostra `2998.8 MHz`.

## Processo clean room

1. **Observação de caixa-preta.** A opção documentada `-txt` do CPU-Z gerou o relatório dele numa máquina de teste. Isso mostra *o que* o CPU-Z exibe e em que formato, não *como* ele faz.
2. **Inventário de funcionalidades.** O readme e os nomes de strings e classes RTTI exportados pelo `cpuz.exe`, inspecionados com o [REA](https://github.com/morluto/rea) e o Ghidra, mostram *quais* funcionalidades e famílias de hardware existem. **Nenhuma função foi descompilada ou traduzida.**
3. **Especificação.** Tudo isso virou o [FEATURES.md](FEATURES.md).
4. **Implementação só com fontes públicas:**
   - CPU e firmware: Intel SDM e datasheets, AMD APM/PPR, DMTF SMBIOS.
   - Memória: anexos SPD da JEDEC, JEP106, XMP e EXPO.
   - Barramentos e telas: especificações PCI/PCIe, VESA EDID.
   - Discos: NVMe e SMART ATA.
   - Chips e GPUs: datasheets dos chips e documentação de NVML, ADL, DXGI, IOKit e sysfs.
   - Documentação da comunidade para interfaces não documentadas mas públicas (layout das células da tabela PM da AMD, IDs de função da NVAPI, rótulos de sensores das placas), usada como fato e reescrita no nosso código.
5. **Verificação por comportamento.** A saída foi comparada com o relatório do CPU-Z no mesmo hardware. Onde há diferença, seguimos a especificação pública.

## Compilando

Requer Rust 1.89+. Não há bibliotecas de sistema para instalar, nem no Linux: a interface carrega X11/Wayland/OpenGL em tempo de execução, e o ícone da bandeja no Linux fala D-Bus em Rust puro.

```bash
git clone https://github.com/joaoregis/cpuzeez
```
```bash
cargo build --release
```
```bash
cargo test
```

O CI ([`ci.yml`](.github/workflows/ci.yml)) compila, testa e roda o binário de verdade em:
- runners nativos de Windows, Ubuntu x64/arm64 e macOS arm64/x64;
- containers Debian, Ubuntu 22.04 e Arch;
- e testa instalação, atualização, bloqueio de downgrade e desinstalação do MSI.

O workflow de release ([`release.yml`](.github/workflows/release.yml)) gera o MSI, o DMG/PKG, o `.deb` e o pacote do Arch a cada tag `v*`, e usa a seção daquela versão do [CHANGELOG](CHANGELOG.md) como notas da release.

## Contribuindo

Issues e PRs são bem-vindos, principalmente:
- **Placas-mãe novas:** use o [modelo de issue de sensores](.github/ISSUE_TEMPLATE/board-sensors.yml) com a saída de `CPUZEEZ_EC_DUMP=1 cpuzeez -txt=placa`.
- **CPUs e GPUs que faltam nas tabelas** (`src/cpu/db.rs`, `src/gpu/db.rs`), citando fonte pública.
- **Relatórios de Macs reais e de sistemas Intel**, principalmente para os timings de memória Intel.
- **Traduções** (tabelas `PT_BR` / `ES` no fim de cada `src/gui/*.rs`; um idioma novo é uma tabela por arquivo).

Mantenha a regra clean room: **nada derivado do binário do CPU-Z**.

Obrigado a [@vinicius741](https://github.com/vinicius741) pelas correções de assinatura no macOS e de memória no Apple Silicon ([#1](https://github.com/joaoregis/cpuzeez/pull/1), [#2](https://github.com/joaoregis/cpuzeez/pull/2)).

## Licença

Licenciado sob [Apache 2.0](LICENSE-APACHE) ou [MIT](LICENSE-MIT), à sua escolha. O PawnIO e seus módulos são projetos separados, com licenças próprias, e não acompanham o CpuZeeZ.
