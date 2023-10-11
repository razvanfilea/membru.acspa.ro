import useExitIfNotFounder from "../../hooks/useExitIfNotFounder";
import {ReactElement, useState} from "react";
import {Button, Card, NumberInput, Space, Stack, Switch, Title} from "@mantine/core";
import useGlobalVars from "../../hooks/useGlobalVars";
import {useSupabaseClient} from "@supabase/auth-helpers-react";
import {Database} from "../../types/database.types";

export default function WebsiteSettingsPage(): ReactElement {
    useExitIfNotFounder();

    const supabase = useSupabaseClient<Database>()

    const [maintenanceMode, setMaintenanceMode] = useState(false);
    const [entranceCode, setEntranceCode] = useState<number | "">("");
    useGlobalVars((vars) => {
        setMaintenanceMode(vars.maintenance)
        setEntranceCode(vars.entrance_code)
    });

    return <Card sx={(theme) => ({margin: theme.spacing.md})}>
        <Stack spacing={'xl'}>
            <Title>Setări website</Title>

            <Switch
                size={'lg'} label={"Mod mentenanță"}
                checked={maintenanceMode}
                onChange={(event) => setMaintenanceMode(event.currentTarget.checked)}/>

            <NumberInput size={'lg'} label={"Cod intrare:"} hideControls={true} value={entranceCode} onChange={setEntranceCode}/>

            <Space h={'xl'} />

            <Button size={'lg'} onClick={async () => {
                await supabase.from('global_vars')
                    .update({
                        maintenance: maintenanceMode,
                        entrance_code: entranceCode === "" ? undefined : entranceCode
                    })
                    .gte('maintenance', 0) // Workaround for supabase not allowing update without a where clause
            }}>Aplică setările</Button>

        </Stack>

    </Card>
}