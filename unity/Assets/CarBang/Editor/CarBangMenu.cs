// Меню редактора: собрать демо-сцену одной кнопкой и подготовить проект (материалы-шаблоны для сборки,
// тени и сглаживание URP). Импорт моделей Kenney с Read/Write для вмятин.
using System.IO;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.Rendering;
using UnityEngine.Rendering.Universal;

namespace CarBang.EditorTools
{
    public static class CarBangMenu
    {
        const string ScenePath = "Assets/CarBang/Scenes/CarBangDemo.unity";
        const string MatDir = "Assets/CarBang/Resources/CarBangMats";

        [MenuItem("CarBang/Собрать демо-сцену", false, 1)]
        public static void BuildDemoScene()
        {
            if (!EditorSceneManager.SaveCurrentModifiedScenesIfUserWantsTo()) return;
            PrepareProject();
            var scene = EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
            var go = new GameObject("CarBangDemo");
            go.AddComponent<DemoBootstrap>();
            var sky = AssetDatabase.LoadAssetAtPath<Material>(MatDir + "/Sky.mat");
            if (sky != null) RenderSettings.skybox = sky;
            Directory.CreateDirectory(Path.GetDirectoryName(ScenePath));
            EditorSceneManager.SaveScene(scene, ScenePath);
            var list = new System.Collections.Generic.List<EditorBuildSettingsScene> { new EditorBuildSettingsScene(ScenePath, true) };
            foreach (var s in EditorBuildSettings.scenes) if (s.path != ScenePath) list.Add(s);
            EditorBuildSettings.scenes = list.ToArray();
            Debug.Log("[CarBang] Демо-сцена собрана: " + ScenePath + ". Нажмите Play.");
        }

        [MenuItem("CarBang/Переимпортировать модели машин", false, 20)]
        public static void ReimportModels()
        {
            foreach (var dir in new[] { "Assets/CarBang/Resources/CarBangCars", "Assets/CarBang/Resources/CarBangProps" })
            {
                if (!AssetDatabase.IsValidFolder(dir)) continue;
                foreach (var guid in AssetDatabase.FindAssets("t:Model", new[] { dir }))
                    AssetDatabase.ImportAsset(AssetDatabase.GUIDToAssetPath(guid), ImportAssetOptions.ForceUpdate);
            }
            Debug.Log("[CarBang] Модели переимпортированы с Read/Write");
        }

        /// <summary>Шаблоны материалов держат ссылки на шейдеры, иначе сборка может их выкинуть.</summary>
        static void PrepareProject()
        {
            Directory.CreateDirectory(MatDir);
            foreach (var name in new[] { "Universal Render Pipeline/Lit", "Universal Render Pipeline/Unlit", "Universal Render Pipeline/Particles/Unlit" })
            {
                var path = MatDir + "/" + name.Replace('/', '_') + ".mat";
                if (AssetDatabase.LoadAssetAtPath<Material>(path) != null) continue;
                var sh = Shader.Find(name);
                if (sh == null) { Debug.LogError("[CarBang] Нет шейдера " + name + ". Проект должен быть на URP (шаблон Universal 3D)."); continue; }
                AssetDatabase.CreateAsset(new Material(sh), path);
            }
            var skyPath = MatDir + "/Sky.mat";
            if (AssetDatabase.LoadAssetAtPath<Material>(skyPath) == null)
            {
                var sh = Shader.Find("Skybox/Procedural");
                if (sh != null) AssetDatabase.CreateAsset(new Material(sh), skyPath);
            }

            // URP: тени до 150 м в 2 каскада, MSAA 4×, мягкие тени
            // активный ассет с учётом уровня качества (в шаблоне URP у каждого уровня свой)
            var urp = GraphicsSettings.currentRenderPipeline as UniversalRenderPipelineAsset;
            if (urp != null)
            {
                Undo.RecordObject(urp, "CarBang URP settings");
                urp.shadowDistance = 150f;
                urp.shadowCascadeCount = 2;
                urp.msaaSampleCount = 4;
                EditorUtility.SetDirty(urp);
            }
            else Debug.LogWarning("[CarBang] Не найден URP Asset в Graphics Settings: проверьте, что проект создан из шаблона Universal 3D");
            AssetDatabase.SaveAssets();
        }
    }

    /// <summary>Модели Kenney в Resources/CarBangCars и CarBangProps: Read/Write (для вмятин), без своих материалов.</summary>
    public class CarModelPostprocessor : AssetPostprocessor
    {
        void OnPreprocessModel()
        {
            if (!assetPath.Contains("/CarBangCars/") && !assetPath.Contains("/CarBangProps/")) return;
            var mi = (ModelImporter)assetImporter;
            mi.isReadable = true;
            mi.materialImportMode = ModelImporterMaterialImportMode.None;
            mi.importAnimation = false;
            mi.importCameras = false;
            mi.importLights = false;
        }
    }
}
