package main

import (
	"embed"
	"log"
	"os"

	"Oj-Agent/llm"

	"github.com/wailsapp/wails/v3/pkg/application"
)

//go:embed all:frontend/dist
var assets embed.FS

func main() {
	llmClient, err := llm.NewClient(nil)
	if err != nil {
		log.Printf("LLM init error: %v", err)
	}

	if llmClient != nil && llmClient.Available() {
		model := os.Getenv("DEEPSEEK_MODEL")
		if model == "" { model = os.Getenv("LLM_MODEL") }
		if model == "" { model = "deepseek-chat" }
		log.Printf("LLM: DeepSeek enabled (%s)", model)
	} else {
		log.Println("LLM: no API key configured, using mock data")
	}

	chatService := NewChatService(llmClient)

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
		Title:            "OJ Agent - 算法题解动画",
		Width:            1400,
		Height:           850,
		MinWidth:         1000,
		MinHeight:        600,
		BackgroundColour: application.NewRGB(10, 14, 23),
		URL:              "/",
	})

	err = app.Run()
	if err != nil {
		log.Fatal(err)
	}
}

func maskKey(key string) string {
	if len(key) <= 8 {
		return "***"
	}
	return key[:4] + "****" + key[len(key)-4:]
}
