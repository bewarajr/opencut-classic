import type { EditorCore } from "@/core";
import { usePanelStore } from "@/editor/panel-store";
import { usePreviewStore } from "@/preview/preview-store";
import { useSoundsStore } from "@/sounds/sounds-store";
import { useStickersStore } from "@/stickers/stickers-store";
import { useAssetsPanelStore } from "@/components/editor/panels/assets/assets-panel-store";
import { usePropertiesStore } from "@/components/editor/panels/properties/stores/properties-store";
import { useKeybindingsStore } from "@/actions/keybindings-store";
import { useTimelineStore } from "@/timeline/timeline-store";
import { storageService } from "@/services/storage/service";

export function exposeOpenCutMcpBridge({
	editor,
}: {
	editor: EditorCore;
}): () => void {
	if (process.env.NEXT_PUBLIC_OPENCUT_MCP_ENABLED !== "true") {
		return () => {};
	}

	const targetWindow = window as Window & {
		__opencut?: EditorCore;
		__stores?: Record<string, unknown>;
		__opencutStorageBridge?: { getSerializedProject: (id: string) => Promise<unknown> };
	};
	const stores = {
		project: {
			getState: () => ({
				projects: editor.project.getSavedProjects(),
				activeProjectId:
					editor.project.getActiveOrNull()?.metadata.id ?? null,
			}),
		},
		panel: usePanelStore,
		preview: usePreviewStore,
		sounds: useSoundsStore,
		stickers: useStickersStore,
		assets: useAssetsPanelStore,
		properties: usePropertiesStore,
		keybindings: useKeybindingsStore,
		timeline: useTimelineStore,
	};

	targetWindow.__opencut = editor;
	targetWindow.__stores = stores;
	targetWindow.__opencutStorageBridge = {
		getSerializedProject: (id) => storageService.getSerializedProject({ id }),
	};

	return () => {
		if (targetWindow.__opencut === editor) {
			delete targetWindow.__opencut;
		}
		if (targetWindow.__stores === stores) {
			delete targetWindow.__stores;
		}
		delete targetWindow.__opencutStorageBridge;
	};
}
