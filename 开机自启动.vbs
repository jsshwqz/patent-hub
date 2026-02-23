Set WshShell = CreateObject("WScript.Shell")
' 获取脚本所在目录
strPath = CreateObject("Scripting.FileSystemObject").GetParentFolderName(WScript.ScriptFullName)
' 后台启动服务器（不显示窗口）
WshShell.Run """" & strPath & "\target\release\patent-hub.exe""", 0, False
