using System;
using UnityEditor;
using UnityEditor.Build.Reporting;
using UnityEditor.SceneManagement;
using UnityEngine;

public static class BuildFixture
{
    public static void Build()
    {
        EditorSceneManager.NewScene(NewSceneSetup.EmptyScene);
        new GameObject("Runtime correctness fixture").AddComponent<RuntimeState>();
        EditorSceneManager.SaveScene(EditorSceneManager.GetActiveScene(), "Assets/Fixture.unity");
        PlayerSettings.companyName = "ReverseboltResearch";
        PlayerSettings.productName = "Runtime object fixture";
        PlayerSettings.runInBackground = true;
        PlayerSettings.SetScriptingBackend(BuildTargetGroup.Standalone, ScriptingImplementation.IL2CPP);
        PlayerSettings.SetManagedStrippingLevel(BuildTargetGroup.Standalone, ManagedStrippingLevel.Low);
        var result = BuildPipeline.BuildPlayer(new BuildPlayerOptions {
            scenes = new[] { "Assets/Fixture.unity" },
            locationPathName = Environment.GetEnvironmentVariable("REVERSEBOLT_FIXTURE_BUILD"),
            target = BuildTarget.StandaloneWindows64,
            options = BuildOptions.None
        });
        if (result.summary.result != BuildResult.Succeeded) throw new Exception("Fixture build failed: " + result.summary.result);
    }
}
