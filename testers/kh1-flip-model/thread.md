# KH1: can we write models KH1 loads? (flip test, step 1)

> [!tip] Assistant
> **What this is:** the first test from your option (b) ("would it be easier to bring Re:CoM's
> maps and bosses into KH1?"). Before anything from Re:CoM can go into KH1, we need to write KH1
> model files ourselves and have KH1 load them with its own loader. This test is about KH1 only.
> Nothing changes in Re:CoM.
>
> **What I built:** a writer for KH1's model files and for the PC's HD meshes (the meshes the PC
> actually draws). On your install, all 500 KH1 models and all 941 HD meshes come back byte for
> byte after reading and writing them.
>
> I made two test files for the **Shadow**. Each is a complete package for Panacea, with the
> model and its HD mesh written by our code:
> - `rebuild`: the same Shadow, rebuilt by our writer. It should look exactly as normal.
> - `big`: every point of the Shadow's mesh moved 1.5× further from its bone, so it looks
>   puffed up. It's ugly on purpose, so you can tell it's ours.
>
> They're already on your PC, in
> `C:\Users\PM\source\repos\kh2-combat\local\flip-test\rebuild\kh1\raw\xa_ex_2020.mdls` and
> `...\flip-test\big\kh1\raw\xa_ex_2020.mdls`.
>
> **Please check, in order:**
> 1. **Rebuild.** Copy the `rebuild` file to
>    `D:\Games\[2] Emulators & Randomizers\Archipelago\KH1 Stuff\openkh\mod\kh1\raw\xa_ex_2020.mdls`
>    (create the `raw` folder if it isn't there). Start KH1 without rebuilding mods in the Mods
>    Manager, since a rebuild may clear that folder. Load a save in Traverse Town and find
>    Shadows.
>    - Do they look and move normally? Do they fight and die normally?
>    - Panacea's console (you have `debug_log=true`) should print a `LoadFile(...raw\xa_ex_2020.mdls...)`
>      line when they appear. Please send that line or a screenshot.
> 2. **Big.** Quit, copy the `big` file over the same path, and start again. Do the Shadows look
>    puffed up, with normal textures? A screenshot would be ideal.
> 3. **Clean up.** Delete `mod\kh1\raw\xa_ex_2020.mdls` when you're done.
>
> If the game crashes or the Shadows are invisible at step 1, that's a real result too. Please
> tell me what happened and when.
>
> **What the result means:**
> - Step 1 works: KH1 accepts a model package written entirely by our code, through its own
>   loader.
> - Step 2 works: KH1 draws mesh data that our writer changed.
> - Both work: step 2 of the test comes next, putting a Re:CoM Organization model through the
>   same writer.
>
> **Update (owner: "copy all the files and stuff for me"):** I've copied the `rebuild` file into
> `mod\kh1\raw\xa_ex_2020.mdls` for you, so step 1 is ready. Tell me when you've tried it and
> I'll swap in `big` for step 2, then remove it after.
> Your mod folder also has a Shadow texture override from another mod
> (`mod\kh1\remastered\xa_ex_2020.mdls\-1c0..-1c2.dds`). While our file is in place, Panacea
> should take the Shadow's textures from our file (KH1's own), not from that folder. That's
> from reading Panacea's source, not seen yet. So the Shadows may look like the unmodded game
> during this test. That's expected.
>
> 1:1 check: checked the writer against every KH1 model and HD mesh in your install, byte for
> byte (files), and Panacea's raw-file route against OpenKH's source. Couldn't check whether KH1
> loads and draws the files, because that needs the game running. That's this test.
