# -*- coding: utf-8 -*-
"""Tests de l'option TFAR de publier.py (detection du .ts3_plugin, version, empreinte de la DLL).

    python -m unittest discover -s serveur/publier -v
"""
import contextlib
import hashlib
import io
import os
import tempfile
import unittest
import zipfile

import publier


def plugin(chemin, dll, contenu=b"dll"):
    os.makedirs(os.path.dirname(chemin), exist_ok=True)
    with zipfile.ZipFile(chemin, "w") as z:
        z.writestr("package.ini", "Name = Task Force Arrowhead Radio\n")
        z.writestr("plugins/" + dll, contenu)
        z.writestr("plugins/radio-sounds/ab/local_start.wav", b"")
    return chemin


class Tfar(unittest.TestCase):
    def setUp(self):
        self.dossier = tempfile.TemporaryDirectory()
        racine = self.dossier.name
        self.mods = os.path.join(racine, "mods")
        self.config = os.path.join(racine, "config")
        os.makedirs(os.path.join(self.mods, "@CBA_A3"))
        os.makedirs(self.config)
        self.anciens = publier.MODS, publier.CONFIG
        publier.MODS, publier.CONFIG = self.mods, self.config

    def tearDown(self):
        publier.MODS, publier.CONFIG = self.anciens
        self.dossier.cleanup()

    def resoudre(self, bloc, mods=("@CBA_A3", "@task_force_radio", "@TFAR")):
        config = {} if bloc is None else {"tfar": bloc}
        reglage = publier.lire_tfar(config)
        presents = [m for m in mods if os.path.isdir(os.path.join(self.mods, m))]
        sortie = io.StringIO()
        with contextlib.redirect_stdout(sortie):
            infos = publier.infos_tfar(reglage, publier.plugin_tfar(reglage, presents))
        return infos, sortie.getvalue()

    def test_inactif_rien_n_est_publie(self):
        plugin(os.path.join(self.mods, "@task_force_radio", "teamspeak", "task_force_radio.ts3_plugin"), "task_force_radio_win64.dll")
        infos, _ = self.resoudre({"actif": False, "version": "stable", "plugin": ""})
        self.assertIsNone(infos)
        publiee = publier.config_publiee({"version": 1, "tfar": {"actif": False}}, ["@CBA_A3"], infos)
        self.assertEqual(sorted(publiee), ["mods", "version"])

    def test_stable_detectee_dans_le_mod(self):
        chemin = plugin(os.path.join(self.mods, "@task_force_radio", "teamspeak", "task_force_radio.ts3_plugin"), "task_force_radio_win64.dll", b"stable")
        infos, _ = self.resoudre({"actif": True, "version": "stable", "plugin": ""})
        self.assertEqual(infos["chemin"], chemin)
        self.assertEqual(infos["version"], "stable")
        self.assertEqual(infos["dll"], hashlib.sha256(b"stable").hexdigest())
        publiee = publier.config_publiee({"version": 1, "tfar": {"actif": True}}, ["@task_force_radio"], infos)
        self.assertEqual(publiee["tfarPlugin"], "tfar/task_force_radio.ts3_plugin")
        self.assertEqual(publiee["tfarVersion"], "stable")
        self.assertEqual(publiee["tfarDll"], hashlib.sha256(b"stable").hexdigest())
        self.assertNotIn("tfar", publiee)

    def test_beta_detectee_et_version_corrigee(self):
        plugin(os.path.join(self.mods, "@TFAR", "teamspeak", "task_force_radio.ts3_plugin"), "TFAR_win64.dll", b"beta")
        infos, sortie = self.resoudre({"actif": True, "version": "stable", "plugin": ""})
        self.assertEqual(infos["version"], "beta")
        self.assertEqual(infos["dll"], hashlib.sha256(b"beta").hexdigest())
        self.assertIn("tfar.version", sortie)

    def test_plusieurs_plugins_differents_refuses(self):
        plugin(os.path.join(self.mods, "@task_force_radio", "teamspeak", "task_force_radio.ts3_plugin"), "task_force_radio_win64.dll", b"a")
        plugin(os.path.join(self.mods, "@TFAR", "teamspeak", "task_force_radio.ts3_plugin"), "TFAR_win64.dll", b"b")
        with contextlib.redirect_stderr(io.StringIO()) as erreur, self.assertRaises(SystemExit):
            self.resoudre({"actif": True, "version": "beta", "plugin": ""})
        self.assertIn("tfar.plugin", erreur.getvalue())

    def test_copies_identiques_acceptees(self):
        a = plugin(os.path.join(self.mods, "@task_force_radio", "teamspeak", "task_force_radio.ts3_plugin"), "task_force_radio_win64.dll")
        plugin(os.path.join(self.mods, "@task_force_radio", "copie", "task_force_radio.ts3_plugin"), "task_force_radio_win64.dll")
        os.utime(a)
        infos, _ = self.resoudre({"actif": True, "version": "stable", "plugin": ""})
        self.assertEqual(infos["version"], "stable")

    def test_chemin_explicite_dans_config(self):
        plugin(os.path.join(self.config, "tfar", "mon.ts3_plugin"), "TFAR_win64.dll")
        plugin(os.path.join(self.mods, "@task_force_radio", "teamspeak", "task_force_radio.ts3_plugin"), "task_force_radio_win64.dll")
        infos, _ = self.resoudre({"actif": True, "version": "beta", "plugin": "tfar/mon.ts3_plugin"})
        self.assertEqual(infos["nom"], "mon.ts3_plugin")
        self.assertEqual(infos["version"], "beta")

    def test_actif_sans_plugin_refuse(self):
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            self.resoudre({"actif": True, "version": "stable", "plugin": ""})

    def test_reglages_invalides_refuses(self):
        for bloc in ({"actif": True, "version": "1.0"}, {"actif": "oui"}, {"actif": True, "plugin": "../x.ts3_plugin"}, "stable"):
            with self.subTest(bloc=bloc), contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                publier.lire_tfar({"tfar": bloc})

    def test_ancien_format_plugin_a_la_racine(self):
        plugin(os.path.join(self.mods, "task_force_radio.ts3_plugin"), "TFAR_win64.dll")
        infos, sortie = self.resoudre(None)
        self.assertEqual(infos["version"], "beta")
        self.assertIn("bloc", sortie)
        os.remove(os.path.join(self.mods, "task_force_radio.ts3_plugin"))
        self.assertIsNone(self.resoudre(None)[0])

    def test_plugin_illisible(self):
        chemin = os.path.join(self.mods, "@task_force_radio", "teamspeak", "task_force_radio.ts3_plugin")
        os.makedirs(os.path.dirname(chemin))
        with open(chemin, "wb") as f:
            f.write(b"pas un zip")
        infos, sortie = self.resoudre({"actif": True, "version": "stable", "plugin": ""})
        self.assertEqual((infos["version"], infos["dll"]), ("stable", ""))
        self.assertIn("ne pourra pas verifier", sortie)


class Installeur(unittest.TestCase):
    def setUp(self):
        self.dossier = tempfile.TemporaryDirectory()
        self.build = os.path.join(self.dossier.name, "launcher-build")
        self.donnees = os.path.join(self.dossier.name, "donnees")
        os.makedirs(self.build)
        self.anciens = publier.LAUNCHER_BUILD, publier.DONNEES, publier.URL_PUBLIQUE
        publier.LAUNCHER_BUILD, publier.DONNEES, publier.URL_PUBLIQUE = self.build, self.donnees, "https://play.example.com"
        self.exe = os.path.join(self.build, "Mon Launcher_0.2.0_x64-setup.exe")
        with open(self.exe, "wb") as f:
            f.write(b"MZ")

    def tearDown(self):
        publier.LAUNCHER_BUILD, publier.DONNEES, publier.URL_PUBLIQUE = self.anciens
        self.dossier.cleanup()

    def test_sans_signature_publie_le_telechargement_sans_latest(self):
        os.makedirs(os.path.join(self.donnees, "update"))
        with open(os.path.join(self.donnees, "update", "latest.json"), "w") as f:
            f.write("{}")
        maj = publier.preparer_launcher(publier.installeur_local(), "")
        self.assertFalse(maj["signe"])
        self.assertTrue(maj["a_publier"])
        publier.publier_launcher(maj)
        update = os.path.join(self.donnees, "update")
        self.assertEqual(sorted(os.listdir(update)), ["Mon-Launcher_0.2.0_x64-setup.exe", publier.INSTALLEUR_STABLE])
        self.assertFalse(publier.preparer_launcher(publier.installeur_local(), "")["a_publier"])

    def test_signe_ecrit_latest(self):
        with open(self.exe + ".sig", "w") as f:
            f.write("signature")
        maj = publier.preparer_launcher(publier.installeur_local(), "notes")
        self.assertTrue(maj["signe"])
        publier.publier_launcher(maj)
        latest = publier.lire_json(os.path.join(self.donnees, "update", "latest.json"))
        self.assertEqual(latest["version"], "0.2.0")
        self.assertEqual(latest["platforms"]["windows-x86_64"]["signature"], "signature")
        self.assertFalse(publier.preparer_launcher(publier.installeur_local(), "notes")["a_publier"])


if __name__ == "__main__":
    unittest.main()
