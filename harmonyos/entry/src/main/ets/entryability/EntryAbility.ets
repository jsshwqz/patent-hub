import { AbilityConstant, UIAbility, Want } from '@kit.AbilityKit';
import { hilog } from '@kit.PerformanceAnalysisKit';
import { window } from '@kit.ArkUI';

const TAG: string = 'PatentHub';
const DOMAIN: number = 0x0000;

export default class EntryAbility extends UIAbility {
  onCreate(want: Want, launchParam: AbilityConstant.LaunchParam): void {
    hilog.info(DOMAIN, TAG, 'Ability onCreate');
  }

  onDestroy(): void {
    hilog.info(DOMAIN, TAG, 'Ability onDestroy');
  }

  onWindowStageCreate(windowStage: window.WindowStage): void {
    hilog.info(DOMAIN, TAG, 'Ability onWindowStageCreate');

    // 设置全屏沉浸式模式
    windowStage.getMainWindow().then((mainWindow: window.Window) => {
      // 设置全屏
      mainWindow.setWindowLayoutFullScreen(true).then(() => {
        hilog.info(DOMAIN, TAG, 'Window set to fullscreen');
      }).catch((err: Error) => {
        hilog.error(DOMAIN, TAG, 'Failed to set fullscreen: %{public}s', err.message);
      });

      // 设置状态栏和导航栏样式（深色主题）
      const sysBarProps: window.SystemBarProperties = {
        statusBarColor: '#0d1117',
        statusBarContentColor: '#c9d1d9',
        navigationBarColor: '#0d1117',
        navigationBarContentColor: '#c9d1d9'
      };
      mainWindow.setWindowSystemBarProperties(sysBarProps).then(() => {
        hilog.info(DOMAIN, TAG, 'System bar properties set');
      }).catch((err: Error) => {
        hilog.error(DOMAIN, TAG, 'Failed to set system bar: %{public}s', err.message);
      });
    }).catch((err: Error) => {
      hilog.error(DOMAIN, TAG, 'Failed to get main window: %{public}s', err.message);
    });

    // 加载启动页
    windowStage.loadContent('pages/Splash', (err) => {
      if (err.code) {
        hilog.error(DOMAIN, TAG, 'Failed to load Splash: %{public}s', JSON.stringify(err));
        // 降级直接加载主页
        windowStage.loadContent('pages/Index');
        return;
      }
      hilog.info(DOMAIN, TAG, 'Splash page loaded');
    });
  }

  onWindowStageDestroy(): void {
    hilog.info(DOMAIN, TAG, 'Ability onWindowStageDestroy');
  }

  onForeground(): void {
    hilog.info(DOMAIN, TAG, 'Ability onForeground');
  }

  onBackground(): void {
    hilog.info(DOMAIN, TAG, 'Ability onBackground');
  }
}
