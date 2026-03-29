import { router } from '@kit.ArkUI';

@Entry
@Component
struct Splash {
  @State logoOpacity: number = 0;
  @State titleOpacity: number = 0;
  @State subtitleOpacity: number = 0;
  private timerId: number = -1;

  aboutToAppear(): void {
    // 分阶段淡入动画
    animateTo({ duration: 600, curve: Curve.EaseOut }, () => {
      this.logoOpacity = 1;
    });

    setTimeout(() => {
      animateTo({ duration: 500, curve: Curve.EaseOut }, () => {
        this.titleOpacity = 1;
      });
    }, 300);

    setTimeout(() => {
      animateTo({ duration: 500, curve: Curve.EaseOut }, () => {
        this.subtitleOpacity = 1;
      });
    }, 600);

    // 2 秒后跳转主页
    this.timerId = setTimeout(() => {
      router.replaceUrl({
        url: 'pages/Index'
      }).catch((err: Error) => {
        console.error('[PatentHub] 跳转主页失败:', err.message);
      });
    }, 2000);
  }

  aboutToDisappear(): void {
    if (this.timerId !== -1) {
      clearTimeout(this.timerId);
      this.timerId = -1;
    }
  }

  build() {
    Column() {
      Blank()
        .layoutWeight(1)

      // Logo
      Column() {
        Text('🔍')
          .fontSize(80)
          .opacity(this.logoOpacity)

        Text('Patent Hub')
          .fontSize(36)
          .fontWeight(FontWeight.Bold)
          .fontColor('#58a6ff')
          .margin({ top: 20 })
          .opacity(this.titleOpacity)

        Text('专利中心')
          .fontSize(18)
          .fontColor('#c9d1d9')
          .margin({ top: 4 })
          .opacity(this.titleOpacity)

        Text('智能专利检索与创新分析')
          .fontSize(14)
          .fontColor('#8b949e')
          .margin({ top: 12 })
          .opacity(this.subtitleOpacity)
      }
      .alignItems(HorizontalAlign.Center)

      Blank()
        .layoutWeight(1)

      // 底部版本信息
      Column() {
        Text('Powered by Patent Hub')
          .fontSize(12)
          .fontColor('#484f58')
        Text('v0.3.5')
          .fontSize(11)
          .fontColor('#30363d')
          .margin({ top: 4 })
      }
      .alignItems(HorizontalAlign.Center)
      .margin({ bottom: 40 })
    }
    .width('100%')
    .height('100%')
    .backgroundColor('#0d1117')
    .justifyContent(FlexAlign.Center)
  }
}
