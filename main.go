package main

import (
	"embed"
	"log"

	"github.com/wailsapp/wails/v3/pkg/application"
)

//go:embed all:frontend/dist
var assets embed.FS

func main() {
	chatService := NewChatService()

	app := application.New(application.Options{
		Name:        "OJ Agent",
		Description: "输入算法题目，自动生成题解动画",
		Services: []application.Service{
			application.NewService(chatService),
		},
		Assets: application.AssetOptions{
			Handler: application.AssetFileServerFS(assets),
		},
		Mac: application.MacOptions{
			ApplicationShouldTerminateAfterLastWindowClosed: true,
		},
	})

	app.Window.NewWithOptions(application.WebviewWindowOptions{
		Title:  "OJ Agent - 算法题解动画",
		Width:  1400,
		Height: 850,
		MinWidth:  1000,
		MinHeight: 600,
		BackgroundColour: application.NewRGB(17, 24, 39),
		URL:              "/",
	})

	err := app.Run()
	if err != nil {
		log.Fatal(err)
	}
}
